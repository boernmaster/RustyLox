//! UDP listener for local MQTT input on port 11884

use crate::GatewayMessage;
use rustylox_core::{Error, Result};
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// UDP listener for local MQTT messages
pub struct UdpListener {
    socket: UdpSocket,
}

impl UdpListener {
    /// Create a new UDP listener
    pub fn new(port: u16) -> Result<Self> {
        let addr = format!("0.0.0.0:{}", port);
        info!("Creating UDP listener on {}", addr);

        let socket = std::net::UdpSocket::bind(&addr)
            .map_err(|e| Error::gateway(format!("Failed to bind UDP socket to {}: {}", addr, e)))?;

        socket
            .set_nonblocking(true)
            .map_err(|e| Error::gateway(format!("Failed to set socket non-blocking: {}", e)))?;

        let socket = UdpSocket::from_std(socket)
            .map_err(|e| Error::gateway(format!("Failed to create tokio UDP socket: {}", e)))?;

        Ok(Self { socket })
    }

    /// Run the UDP listener
    pub async fn run(&self, tx: broadcast::Sender<GatewayMessage>) -> Result<()> {
        info!(
            "UDP listener started on port {}",
            self.socket.local_addr().unwrap()
        );

        let mut buf = vec![0u8; 65535]; // Max UDP packet size

        loop {
            match self.socket.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    debug!("UDP received {} bytes from {}", len, addr);

                    let data = &buf[..len];
                    if let Err(e) = self.process_udp_message(data, &tx) {
                        warn!("Failed to process UDP message: {}", e);
                    }
                }
                Err(e) => {
                    warn!("UDP receive error: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Process a UDP message
    fn process_udp_message(
        &self,
        data: &[u8],
        tx: &broadcast::Sender<GatewayMessage>,
    ) -> Result<()> {
        let content = String::from_utf8_lossy(data);
        let content = content.trim();

        // Parse JSON format: {"topic": "home/sensor", "value": "123"}
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
            let topic = json
                .get("topic")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::gateway("Missing 'topic' in UDP JSON message"))?;

            let value = json.get("value").and_then(|v| v.as_str()).unwrap_or("");

            debug!("UDP message: {} = {}", topic, value);

            let msg = GatewayMessage::UdpReceived {
                topic: topic.to_string(),
                value: value.to_string(),
            };

            tx.send(msg)
                .map_err(|e| Error::gateway(format!("Failed to send UDP message: {}", e)))?;

            return Ok(());
        }

        // Try simple format: topic=value
        if let Some((topic, value)) = content.split_once('=') {
            debug!("UDP message (simple): {} = {}", topic, value);

            let msg = GatewayMessage::UdpReceived {
                topic: topic.to_string(),
                value: value.to_string(),
            };

            tx.send(msg)
                .map_err(|e| Error::gateway(format!("Failed to send UDP message: {}", e)))?;

            return Ok(());
        }

        warn!("Invalid UDP message format: {}", content);
        Err(Error::gateway("Invalid UDP message format"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_udp_json_parsing() {
        let listener = UdpListener::new(0).unwrap(); // Bind to random port
        let (tx, _rx) = broadcast::channel(10);

        let json = r#"{"topic": "home/temperature", "value": "23.5"}"#;
        listener.process_udp_message(json.as_bytes(), &tx).unwrap();
    }

    #[tokio::test]
    async fn test_udp_simple_parsing() {
        let listener = UdpListener::new(0).unwrap();
        let (tx, _rx) = broadcast::channel(10);

        let simple = b"home/humidity=65";
        listener.process_udp_message(simple, &tx).unwrap();
    }
}
