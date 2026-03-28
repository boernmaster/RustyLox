//! MQTT Gateway - Bidirectional relay between MQTT broker and Miniserver
//!
//! This crate handles:
//! - MQTT broker connection and message handling
//! - UDP input listener on port 11884
//! - Message transformation pipeline
//! - Subscription management
//! - Relay to Miniserver via HTTP/UDP

pub mod broker_client;
pub mod relay;
pub mod relay_tracker;
pub mod stats;
pub mod subscription;
pub mod transformer;
pub mod udp_listener;

pub use broker_client::BrokerClient;
pub use relay::Relay;
pub use relay_tracker::{RelayTracker, RelayedTopicsResponse, TopicSettings};
pub use stats::{MqttGatewayStats, RejectedParam, StatsSnapshot};
pub use subscription::{Subscription, SubscriptionManager};
pub use transformer::{TransformResult, Transformer, TransformerRegistry};
pub use udp_listener::UdpListener;

use rustylox_config::GeneralConfig;
use rustylox_core::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info};

/// Gateway message types
#[derive(Debug, Clone)]
pub enum GatewayMessage {
    /// MQTT message received from broker
    MqttReceived { topic: String, payload: Vec<u8> },
    /// UDP message received
    UdpReceived { topic: String, value: String },
    /// Message transformed and ready for relay
    ReadyForRelay {
        topic: String,
        value: String,
        relay_to_miniserver: bool,
        relay_to_mqtt: bool,
    },
}

/// MQTT Gateway orchestrator
pub struct MqttGateway {
    config: Arc<RwLock<GeneralConfig>>,
    lbhomedir: PathBuf,
    broker_client: Arc<BrokerClient>,
    udp_listener: Arc<UdpListener>,
    subscription_manager: Arc<SubscriptionManager>,
    transformer_registry: Arc<TransformerRegistry>,
    relay: Arc<Relay>,
    stats: Arc<MqttGatewayStats>,
    relay_tracker: Arc<RelayTracker>,
    message_tx: broadcast::Sender<GatewayMessage>,
}

impl MqttGateway {
    /// Create a new MQTT gateway
    pub fn new(config: Arc<RwLock<GeneralConfig>>, lbhomedir: PathBuf) -> Result<Self> {
        info!("Initializing MQTT Gateway");

        let (message_tx, _) = broadcast::channel(1000);

        // Get MQTT config for broker client
        let mqtt_config = tokio::task::block_in_place(|| {
            let config = config.blocking_read();
            config.mqtt.clone()
        });

        // Initialize components
        let broker_client = Arc::new(BrokerClient::new(&mqtt_config)?);
        let udp_listener = Arc::new(UdpListener::new(11884)?);

        let subscription_manager =
            Arc::new(SubscriptionManager::new(lbhomedir.join("config/system")));

        let transformer_registry = Arc::new(TransformerRegistry::new(
            lbhomedir.join("bin/mqtt/transform"),
        ));

        let stats = Arc::new(MqttGatewayStats::new());
        let relay_tracker = Arc::new(RelayTracker::new());
        let relay = Arc::new(Relay::new(
            Arc::clone(&config),
            Arc::clone(&stats),
            Arc::clone(&relay_tracker),
        ));

        Ok(Self {
            config,
            lbhomedir,
            broker_client,
            udp_listener,
            subscription_manager,
            transformer_registry,
            relay,
            stats,
            relay_tracker,
            message_tx,
        })
    }

    /// Start the gateway (non-blocking)
    pub async fn start(&self) -> Result<()> {
        info!("Starting MQTT Gateway");

        // Load subscriptions
        self.subscription_manager.load().await?;
        info!("Loaded {} subscriptions", self.subscription_manager.count());

        // Load transformers
        self.transformer_registry.load().await?;
        info!("Loaded {} transformers", self.transformer_registry.count());

        // Subscribe to topics on broker
        let subscriptions = self.subscription_manager.get_all();
        for sub in subscriptions {
            self.broker_client.subscribe(&sub.topic).await?;
        }

        // Start broker client
        let broker_handle = {
            let client = Arc::clone(&self.broker_client);
            let tx = self.message_tx.clone();
            tokio::spawn(async move {
                if let Err(e) = client.run(tx).await {
                    error!("Broker client error: {}", e);
                }
            })
        };

        // Start UDP listener
        let udp_handle = {
            let listener = Arc::clone(&self.udp_listener);
            let tx = self.message_tx.clone();
            tokio::spawn(async move {
                if let Err(e) = listener.run(tx).await {
                    error!("UDP listener error: {}", e);
                }
            })
        };

        // Start message processor
        let processor_handle = {
            let gateway = self.clone_arc();
            tokio::spawn(async move {
                if let Err(e) = gateway.process_messages().await {
                    error!("Message processor error: {}", e);
                }
            })
        };

        // Start periodic stats logger (every 5 minutes)
        let stats_logger_handle = {
            let stats = Arc::clone(&self.stats);
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes
                loop {
                    interval.tick().await;
                    Self::log_stats_summary(&stats);
                }
            })
        };

        info!("MQTT Gateway started successfully");

        // Wait for all tasks (in production, these would run forever)
        tokio::try_join!(
            async {
                broker_handle
                    .await
                    .map_err(|e| rustylox_core::Error::gateway(e.to_string()))
            },
            async {
                udp_handle
                    .await
                    .map_err(|e| rustylox_core::Error::gateway(e.to_string()))
            },
            async {
                processor_handle
                    .await
                    .map_err(|e| rustylox_core::Error::gateway(e.to_string()))
            },
            async {
                stats_logger_handle
                    .await
                    .map_err(|e| rustylox_core::Error::gateway(e.to_string()))
            },
        )?;

        Ok(())
    }

    /// Log statistics summary
    fn log_stats_summary(stats: &MqttGatewayStats) {
        let snapshot = stats.snapshot();
        let top_rejected = stats.top_rejected(10);

        if snapshot.messages_received == 0 {
            return; // Don't log if no activity
        }

        info!(
            "MQTT Gateway Summary (last 5min): {} msgs received, {} relayed ({:.1}%), {} filtered, {} accepted by MS ({:.1}% success), {} rejected",
            snapshot.messages_received,
            snapshot.messages_relayed,
            if snapshot.messages_received > 0 { (snapshot.messages_relayed as f64 / snapshot.messages_received as f64) * 100.0 } else { 0.0 },
            snapshot.messages_filtered,
            snapshot.miniserver_accepted,
            snapshot.success_rate(),
            snapshot.miniserver_rejected
        );

        if !top_rejected.is_empty() {
            let top_3: Vec<String> = top_rejected
                .iter()
                .take(3)
                .map(|(name, param)| format!("{} ({}x)", name, param.count))
                .collect();
            info!("Top rejected parameters: {}", top_3.join(", "));
        }
    }

    /// Process incoming messages
    async fn process_messages(&self) -> Result<()> {
        let mut rx = self.message_tx.subscribe();

        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if let Err(e) = self.handle_message(msg).await {
                        error!("Error handling message: {}", e);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    error!("Message processor lagged by {} messages", n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("Message channel closed, stopping processor");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a single message
    async fn handle_message(&self, msg: GatewayMessage) -> Result<()> {
        match msg {
            GatewayMessage::MqttReceived { topic, payload } => {
                self.stats.inc_received();
                let value = String::from_utf8_lossy(&payload).to_string();

                // Apply transformers
                let result = self.transformer_registry.transform(&topic, &value).await?;

                // Check per-topic "do not forward" setting
                if self.relay_tracker.is_do_not_forward(&result.topic) {
                    self.relay_tracker
                        .record_http_cached(&result.topic, &result.value);
                    self.stats.inc_filtered();
                    return Ok(());
                }

                // Relay to Miniserver if configured
                if result.relay_to_miniserver {
                    self.stats.inc_relayed();
                    self.relay
                        .send_to_miniserver(&result.topic, &result.value)
                        .await?;
                } else {
                    self.stats.inc_filtered();
                    self.relay_tracker
                        .record_http_cached(&result.topic, &result.value);
                }
            }
            GatewayMessage::UdpReceived { topic, value } => {
                // Apply transformers (may rewrite topic/value)
                let result = self.transformer_registry.transform(&topic, &value).await?;

                // UDP input always publishes to MQTT — that is the purpose of the UDP gateway
                // (equivalent to original LoxBerry MQTT Gateway UDP interface on port 11884)
                self.broker_client
                    .publish(&result.topic, &result.value)
                    .await?;

                // Also relay to Miniserver if configured
                if result.relay_to_miniserver {
                    self.relay
                        .send_to_miniserver(&result.topic, &result.value)
                        .await?;
                }
            }
            GatewayMessage::ReadyForRelay {
                topic,
                value,
                relay_to_miniserver,
                relay_to_mqtt,
            } => {
                if relay_to_mqtt {
                    self.broker_client.publish(&topic, &value).await?;
                }
                if relay_to_miniserver {
                    self.relay.send_to_miniserver(&topic, &value).await?;
                }
            }
        }

        Ok(())
    }

    /// Clone with Arc wrappers
    fn clone_arc(&self) -> Self {
        Self {
            config: self.config.clone(),
            lbhomedir: self.lbhomedir.clone(),
            broker_client: Arc::clone(&self.broker_client),
            udp_listener: Arc::clone(&self.udp_listener),
            subscription_manager: Arc::clone(&self.subscription_manager),
            transformer_registry: Arc::clone(&self.transformer_registry),
            relay: Arc::clone(&self.relay),
            stats: Arc::clone(&self.stats),
            relay_tracker: Arc::clone(&self.relay_tracker),
            message_tx: self.message_tx.clone(),
        }
    }

    /// Get statistics
    pub fn stats(&self) -> Arc<MqttGatewayStats> {
        Arc::clone(&self.stats)
    }

    /// Get relay tracker for the "Incoming Overview" monitor
    pub fn relay_tracker(&self) -> Arc<RelayTracker> {
        Arc::clone(&self.relay_tracker)
    }

    /// Get gateway status
    pub fn status(&self) -> GatewayStatus {
        GatewayStatus {
            connected: self.broker_client.is_connected(),
            subscriptions: self.subscription_manager.count(),
            transformers: self.transformer_registry.count(),
        }
    }

    /// Reload subscriptions from disk
    pub async fn reload_subscriptions(&self) -> Result<()> {
        info!("Reloading subscriptions");
        self.subscription_manager.load().await?;

        // Re-subscribe on broker
        let subscriptions = self.subscription_manager.get_all();
        for sub in subscriptions {
            self.broker_client.subscribe(&sub.topic).await?;
        }

        Ok(())
    }

    /// Reload transformers from disk
    pub async fn reload_transformers(&self) -> Result<()> {
        info!("Reloading transformers");
        self.transformer_registry.load().await
    }

    /// Subscribe to gateway messages for monitoring
    /// Returns a receiver that gets all MQTT messages passing through the gateway
    pub fn subscribe_messages(&self) -> broadcast::Receiver<GatewayMessage> {
        self.message_tx.subscribe()
    }

    /// Attach a monitor callback so Miniserver sends made by the relay appear in the monitor UI
    pub async fn set_miniserver_monitor(&self, callback: miniserver_client::MonitorCallback) {
        self.relay.set_monitor_callback(callback).await;
    }

    /// Get a clone of the internal message sender.
    ///
    /// External components (e.g. HTTP virtual-input endpoint, Miniserver UDP
    /// receiver) can use this to inject messages into the gateway pipeline so
    /// they are processed exactly like MQTT or UDP-originated messages.
    pub fn message_sender(&self) -> broadcast::Sender<GatewayMessage> {
        self.message_tx.clone()
    }
}

/// Gateway status information
#[derive(Debug, Clone, serde::Serialize)]
pub struct GatewayStatus {
    pub connected: bool,
    pub subscriptions: usize,
    pub transformers: usize,
}
