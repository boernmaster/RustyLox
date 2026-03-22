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

    /// Process a Miniserver-format UDP message: "prefix: key=value key2=value2"
    ///
    /// Each key=value pair is emitted as a separate GatewayMessage.
    /// The MQTT topic is built as `<prefix>/<key>` (lowercased prefix).
    fn process_miniserver_format(
        &self,
        content: &str,
        tx: &broadcast::Sender<GatewayMessage>,
    ) -> Result<()> {
        let content = content.trim();
        let (prefix, kv_part) = if let Some(colon) = content.find(": ") {
            (&content[..colon], &content[colon + 2..])
        } else {
            ("", content)
        };

        let mut count = 0;
        for token in kv_part.split_whitespace() {
            if let Some((key, value)) = token.split_once('=') {
                // Build an MQTT-style topic: prefix/key
                let topic = if prefix.is_empty() {
                    key.to_string()
                } else {
                    format!("{}/{}", prefix, key)
                };

                debug!("UDP Miniserver format: {} = {}", topic, value);

                let msg = GatewayMessage::UdpReceived {
                    topic,
                    value: value.to_string(),
                };

                if let Err(e) = tx.send(msg) {
                    warn!("Failed to send Miniserver UDP message: {}", e);
                }
                count += 1;
            }
        }

        if count == 0 {
            warn!(
                "Miniserver UDP message had prefix but no key=value pairs: {}",
                content
            );
        } else {
            debug!(
                "Processed {} key=value pairs from Miniserver UDP message",
                count
            );
        }

        Ok(())
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
            // Check for Miniserver "prefix: key=value key2=value2" format.
            // If the topic part contains ": " it means the whole payload uses the
            // Loxone Virtual UDP Output pattern.
            if topic.contains(": ") {
                return self.process_miniserver_format(content, tx);
            }

            debug!("UDP message (simple): {} = {}", topic, value);

            let msg = GatewayMessage::UdpReceived {
                topic: topic.to_string(),
                value: value.to_string(),
            };

            tx.send(msg)
                .map_err(|e| Error::gateway(format!("Failed to send UDP message: {}", e)))?;

            return Ok(());
        }

        // Try bare value (Miniserver might send just a string for pulse outputs)
        debug!("UDP message (bare): {}", content);
        let msg = GatewayMessage::UdpReceived {
            topic: content.to_string(),
            value: String::new(),
        };

        tx.send(msg)
            .map_err(|e| Error::gateway(format!("Failed to send UDP message: {}", e)))?;

        Ok(())
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

    #[tokio::test]
    async fn test_udp_miniserver_format_with_prefix() {
        let listener = UdpListener::new(0).unwrap();
        let (tx, mut rx) = broadcast::channel(10);

        let data = b"Weather: Temp=23.5 Humidity=65";
        listener.process_udp_message(data, &tx).unwrap();

        // Should produce two messages
        let msg1 = rx.recv().await.unwrap();
        let msg2 = rx.recv().await.unwrap();

        match msg1 {
            GatewayMessage::UdpReceived { topic, value } => {
                assert_eq!(topic, "Weather/Temp");
                assert_eq!(value, "23.5");
            }
            _ => panic!("Expected UdpReceived"),
        }
        match msg2 {
            GatewayMessage::UdpReceived { topic, value } => {
                assert_eq!(topic, "Weather/Humidity");
                assert_eq!(value, "65");
            }
            _ => panic!("Expected UdpReceived"),
        }
    }

    #[tokio::test]
    async fn test_udp_bare_message() {
        let listener = UdpListener::new(0).unwrap();
        let (tx, mut rx) = broadcast::channel(10);

        let data = b"PULSE";
        listener.process_udp_message(data, &tx).unwrap();

        let msg = rx.recv().await.unwrap();
        match msg {
            GatewayMessage::UdpReceived { topic, value } => {
                assert_eq!(topic, "PULSE");
                assert_eq!(value, "");
            }
            _ => panic!("Expected UdpReceived"),
        }
    }
}
