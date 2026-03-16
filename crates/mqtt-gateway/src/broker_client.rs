//! MQTT broker client using rumqttc

use crate::GatewayMessage;
use loxberry_config::MqttConfig;
use loxberry_core::{Error, Result};
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// MQTT broker client
pub struct BrokerClient {
    client: AsyncClient,
    eventloop: Arc<tokio::sync::Mutex<EventLoop>>,
    connected: Arc<AtomicBool>,
}

impl BrokerClient {
    /// Create a new broker client
    pub fn new(config: &MqttConfig) -> Result<Self> {
        let broker_host = config.broker_host();
        let broker_port = config.broker_port();
        let client_id = format!("loxberry-gateway-{}", std::process::id());

        info!("Connecting to MQTT broker: {}:{}", broker_host, broker_port);

        let mut mqttoptions = MqttOptions::new(&client_id, broker_host, broker_port);
        mqttoptions.set_keep_alive(std::time::Duration::from_secs(30));

        // Set credentials if provided
        if !config.brokeruser.is_empty() {
            info!(
                "Using MQTT broker authentication (user: {})",
                config.brokeruser
            );
            mqttoptions.set_credentials(&config.brokeruser, &config.brokerpass);
        } else {
            info!("Using anonymous MQTT broker connection");
        }

        let (client, eventloop) = AsyncClient::new(mqttoptions, 100);

        Ok(Self {
            client,
            eventloop: Arc::new(tokio::sync::Mutex::new(eventloop)),
            connected: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Run the broker client event loop
    pub async fn run(&self, tx: broadcast::Sender<GatewayMessage>) -> Result<()> {
        let mut eventloop = self.eventloop.lock().await;

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    info!("Connected to MQTT broker");
                    self.connected.store(true, Ordering::Relaxed);
                }
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    debug!("MQTT received: {} = {:?}", publish.topic, publish.payload);

                    let msg = GatewayMessage::MqttReceived {
                        topic: publish.topic.clone(),
                        payload: publish.payload.to_vec(),
                    };

                    if let Err(e) = tx.send(msg) {
                        warn!("Failed to send MQTT message to processor: {}", e);
                    }
                }
                Ok(Event::Incoming(Packet::Disconnect)) => {
                    warn!("Disconnected from MQTT broker");
                    self.connected.store(false, Ordering::Relaxed);
                }
                Ok(Event::Outgoing(_)) => {
                    // Ignore outgoing events
                }
                Ok(_) => {
                    // Ignore other events
                }
                Err(e) => {
                    warn!("MQTT broker unavailable, retrying in 5s: {}", e);
                    self.connected.store(false, Ordering::Relaxed);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Subscribe to a topic
    pub async fn subscribe(&self, topic: &str) -> Result<()> {
        info!("Subscribing to MQTT topic: {}", topic);

        self.client
            .subscribe(topic, QoS::AtLeastOnce)
            .await
            .map_err(|e| Error::gateway(format!("Failed to subscribe to {}: {}", topic, e)))?;

        Ok(())
    }

    /// Publish a message to a topic
    pub async fn publish(&self, topic: &str, payload: &str) -> Result<()> {
        debug!("Publishing to MQTT: {} = {}", topic, payload);

        self.client
            .publish(topic, QoS::AtLeastOnce, false, payload.as_bytes())
            .await
            .map_err(|e| Error::gateway(format!("Failed to publish to {}: {}", topic, e)))?;

        Ok(())
    }

    /// Check if connected to broker
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }
}
