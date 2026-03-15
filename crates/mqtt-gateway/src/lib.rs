//! MQTT Gateway - Bidirectional relay between MQTT broker and Miniserver
//!
//! This crate handles:
//! - MQTT broker connection and message handling
//! - UDP input listener on port 11884
//! - Message transformation pipeline
//! - Subscription management
//! - Relay to Miniserver via HTTP/UDP

pub mod broker_client;
pub mod udp_listener;
pub mod subscription;
pub mod transformer;
pub mod relay;

pub use broker_client::BrokerClient;
pub use udp_listener::UdpListener;
pub use subscription::{SubscriptionManager, Subscription};
pub use transformer::{TransformerRegistry, Transformer, TransformResult};
pub use relay::Relay;

use loxberry_config::MqttConfig;
use loxberry_core::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, error};

/// Gateway message types
#[derive(Debug, Clone)]
pub enum GatewayMessage {
    /// MQTT message received from broker
    MqttReceived {
        topic: String,
        payload: Vec<u8>,
    },
    /// UDP message received
    UdpReceived {
        topic: String,
        value: String,
    },
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
    config: MqttConfig,
    lbhomedir: PathBuf,
    broker_client: Arc<BrokerClient>,
    udp_listener: Arc<UdpListener>,
    subscription_manager: Arc<SubscriptionManager>,
    transformer_registry: Arc<TransformerRegistry>,
    relay: Arc<Relay>,
    message_tx: broadcast::Sender<GatewayMessage>,
}

impl MqttGateway {
    /// Create a new MQTT gateway
    pub fn new(
        config: MqttConfig,
        lbhomedir: PathBuf,
    ) -> Result<Self> {
        info!("Initializing MQTT Gateway");

        let (message_tx, _) = broadcast::channel(1000);

        // Initialize components
        let broker_client = Arc::new(BrokerClient::new(&config)?);
        let udp_listener = Arc::new(UdpListener::new(11884)?);

        let subscription_manager = Arc::new(
            SubscriptionManager::new(lbhomedir.join("config/system"))
        );

        let transformer_registry = Arc::new(
            TransformerRegistry::new(lbhomedir.join("bin/mqtt/transform"))
        );

        let relay = Arc::new(Relay::new());

        Ok(Self {
            config,
            lbhomedir,
            broker_client,
            udp_listener,
            subscription_manager,
            transformer_registry,
            relay,
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

        info!("MQTT Gateway started successfully");

        // Wait for all tasks (in production, these would run forever)
        tokio::try_join!(
            async { broker_handle.await.map_err(|e| loxberry_core::Error::gateway(e.to_string())) },
            async { udp_handle.await.map_err(|e| loxberry_core::Error::gateway(e.to_string())) },
            async { processor_handle.await.map_err(|e| loxberry_core::Error::gateway(e.to_string())) },
        )?;

        Ok(())
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
                let value = String::from_utf8_lossy(&payload).to_string();

                // Apply transformers
                let result = self.transformer_registry
                    .transform(&topic, &value)
                    .await?;

                // Relay to Miniserver if configured
                if result.relay_to_miniserver {
                    self.relay.send_to_miniserver(&result.topic, &result.value).await?;
                }
            }
            GatewayMessage::UdpReceived { topic, value } => {
                // Apply transformers
                let result = self.transformer_registry
                    .transform(&topic, &value)
                    .await?;

                // Publish to MQTT if configured
                if result.relay_to_mqtt {
                    self.broker_client.publish(&result.topic, &result.value).await?;
                }

                // Relay to Miniserver if configured
                if result.relay_to_miniserver {
                    self.relay.send_to_miniserver(&result.topic, &result.value).await?;
                }
            }
            GatewayMessage::ReadyForRelay { topic, value, relay_to_miniserver, relay_to_mqtt } => {
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
            message_tx: self.message_tx.clone(),
        }
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
}

/// Gateway status information
#[derive(Debug, Clone, serde::Serialize)]
pub struct GatewayStatus {
    pub connected: bool,
    pub subscriptions: usize,
    pub transformers: usize,
}
