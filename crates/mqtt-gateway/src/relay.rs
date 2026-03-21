//! Relay messages to Miniserver

use crate::stats::MqttGatewayStats;
use loxberry_config::GeneralConfig;
use loxberry_core::{Error, Result};
use miniserver_client::{MiniserverClient, MonitorCallback};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Message relay to Miniserver
pub struct Relay {
    config: Arc<RwLock<GeneralConfig>>,
    stats: Arc<MqttGatewayStats>,
    /// Cache of Miniserver clients (by Miniserver ID)
    clients: RwLock<HashMap<String, Arc<MiniserverClient>>>,
    /// Optional monitor callback to attach to newly created clients
    monitor_callback: RwLock<Option<MonitorCallback>>,
}

impl Relay {
    /// Create a new relay
    pub fn new(config: Arc<RwLock<GeneralConfig>>, stats: Arc<MqttGatewayStats>) -> Self {
        Self {
            config,
            stats,
            clients: RwLock::new(HashMap::new()),
            monitor_callback: RwLock::new(None),
        }
    }

    /// Set monitor callback so outbound Miniserver calls appear in the monitor UI
    pub async fn set_monitor_callback(&self, callback: MonitorCallback) {
        *self.monitor_callback.write().await = Some(callback);
    }

    /// Get or create a Miniserver client for the given ID
    async fn get_client(&self, ms_id: &str) -> Result<Arc<MiniserverClient>> {
        // Check cache first
        {
            let clients = self.clients.read().await;
            if let Some(client) = clients.get(ms_id) {
                return Ok(Arc::clone(client));
            }
        }

        // Not in cache, create new client
        let config = self.config.read().await;
        let ms_config = config
            .miniserver
            .get(ms_id)
            .ok_or_else(|| Error::miniserver(format!("Miniserver '{}' not found", ms_id)))?
            .clone();
        drop(config);

        let mut client = MiniserverClient::new(ms_config.clone())?;

        // Attach monitor callback so sends appear in the monitor UI
        if let Some(callback) = self.monitor_callback.read().await.as_ref() {
            client.http_mut().set_monitor_callback(callback.clone());
        }

        let client = Arc::new(client);

        // Cache the client
        let mut clients = self.clients.write().await;
        clients.insert(ms_id.to_string(), Arc::clone(&client));

        Ok(client)
    }

    /// Check if a topic should be filtered based on global regex filter
    fn should_filter(&self, topic: &str, filter_pattern: &str) -> bool {
        if filter_pattern.is_empty() {
            return false;
        }

        // Replace slashes with underscores for filtering
        let normalized_topic = topic.replace('/', "_");

        // Try to compile and match regex
        match regex::Regex::new(filter_pattern) {
            Ok(re) => {
                if re.is_match(&normalized_topic) {
                    debug!(
                        "Message filtered: topic '{}' matches global filter pattern '{}'",
                        topic, filter_pattern
                    );
                    true
                } else {
                    false
                }
            }
            Err(e) => {
                warn!("Invalid global regex filter '{}': {}", filter_pattern, e);
                false
            }
        }
    }

    /// Send message to Miniserver via HTTP/UDP
    pub async fn send_to_miniserver(&self, topic: &str, value: &str) -> Result<()> {
        // Check global filter from config
        let filter_pattern = {
            let config = self.config.read().await;
            config.mqtt.topicfilter.clone()
        };

        if self.should_filter(topic, &filter_pattern) {
            debug!(
                "Message FILTERED (not sent to Miniserver): {} = {}",
                topic, value
            );
            return Ok(());
        }

        info!("Relay to Miniserver: {} = {}", topic, value);

        // Get the first configured Miniserver
        // In the future, this should support multiple Miniservers and topic routing
        let config = self.config.read().await;
        let ms_id = if let Some((id, _)) = config.miniserver.iter().next() {
            id.clone()
        } else {
            warn!("No Miniserver configured, cannot relay message");
            return Ok(());
        };
        drop(config);

        // Get or create client
        let client = match self.get_client(&ms_id).await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to get Miniserver client: {}", e);
                return Err(e);
            }
        };

        // Map topic to virtual input parameter name (slashes -> underscores) for HTTP
        let param_name = topic.replace('/', "_");

        // Send to Miniserver via HTTP
        match client
            .send(vec![(param_name.clone(), value.to_string())])
            .await
        {
            Ok(results) => {
                if let Some(&success) = results.get(&param_name) {
                    if success {
                        self.stats.record_accepted();
                        debug!(
                            "Successfully sent {} = {} to Miniserver {}",
                            topic, value, ms_id
                        );
                    } else {
                        self.stats
                            .record_rejected(param_name.clone(), value.to_string());
                        debug!(
                            "Miniserver {} rejected parameter {} (virtual input may not exist)",
                            ms_id, param_name
                        );
                    }
                }
            }
            Err(e) => {
                error!("Failed to send to Miniserver {} via HTTP: {}", ms_id, e);
                return Err(e);
            }
        }

        // Also send via UDP if udpport is configured (old LoxBerry MQTT Gateway format)
        let udp_target = {
            let config = self.config.read().await;
            config.miniserver.get(&ms_id).and_then(|ms| {
                ms.udpport
                    .as_ref()
                    .and_then(|p| p.parse::<u16>().ok())
                    .map(|port| (ms.ipaddress.clone(), port))
            })
        };

        if let Some((ip, port)) = udp_target {
            // Format: "MQTT: topic=value" — matches old LoxBerry MQTT Gateway UDP format
            let msg = format!("MQTT: {}={}", topic, value);
            let target = format!("{}:{}", ip, port);
            match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
                Ok(socket) => {
                    if let Err(e) = socket.send_to(msg.as_bytes(), &target).await {
                        warn!("UDP send to {} failed: {}", target, e);
                    } else {
                        debug!("UDP sent to {}: {}", target, msg);
                    }
                }
                Err(e) => warn!("Failed to bind UDP socket: {}", e),
            }
        }

        Ok(())
    }
}
