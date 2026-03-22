//! UDP receiver for incoming Miniserver data
//!
//! The Loxone Miniserver can send UDP packets to the LoxBerry host
//! (e.g., via "Virtual UDP Output" blocks in the Loxone Config program).
//! This module listens on a configurable UDP port and dispatches
//! received messages so they can be processed or displayed in the
//! communication monitor.

use rustylox_core::{Error, Result};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// Maximum UDP datagram size we accept (Loxone uses ≤ 1024 bytes in practice)
const MAX_DATAGRAM: usize = 4096;

/// A UDP message received from a Miniserver (or any UDP sender)
#[derive(Debug, Clone)]
pub struct UdpMessage {
    /// Source IP:port
    pub from: SocketAddr,
    /// Raw text content of the datagram
    pub payload: String,
    /// Timestamp (Unix seconds, approximate)
    pub timestamp: u64,
}

/// Listens on a UDP port and broadcasts incoming messages.
///
/// Spawn with [`MiniserverUdpReceiver::spawn`] to start listening in the
/// background.  Consumers call [`MiniserverUdpReceiver::subscribe`] to
/// receive a [`broadcast::Receiver<UdpMessage>`].
pub struct MiniserverUdpReceiver {
    tx: broadcast::Sender<UdpMessage>,
    bind_addr: SocketAddr,
}

impl MiniserverUdpReceiver {
    /// Create a new receiver that will listen on `bind_addr`.
    ///
    /// The receiver is not started until [`Self::spawn`] is called.
    ///
    /// # Arguments
    /// * `bind_addr` – local address/port to listen on, e.g. `"0.0.0.0:7700"`
    /// * `channel_capacity` – broadcast channel depth (default: 256)
    pub fn new(bind_addr: SocketAddr, channel_capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(channel_capacity.max(8));
        Self { tx, bind_addr }
    }

    /// Subscribe to incoming UDP messages.
    pub fn subscribe(&self) -> broadcast::Receiver<UdpMessage> {
        self.tx.subscribe()
    }

    /// Spawn the listener task.  Returns immediately; the background task
    /// runs until the process exits or the socket is closed.
    pub async fn spawn(self) -> Result<()> {
        let socket = UdpSocket::bind(self.bind_addr).await.map_err(|e| {
            Error::network(format!(
                "UDP receiver bind failed on {}: {}",
                self.bind_addr, e
            ))
        })?;

        info!("Miniserver UDP receiver listening on {}", self.bind_addr);

        let tx = self.tx;

        tokio::spawn(async move {
            let mut buf = vec![0u8; MAX_DATAGRAM];
            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((n, from)) => {
                        let raw = &buf[..n];
                        let payload = String::from_utf8_lossy(raw).into_owned();
                        debug!("UDP from {}: {}", from, payload.trim());

                        let msg = UdpMessage {
                            from,
                            payload,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0),
                        };

                        // Ignore send errors – no active subscribers is fine
                        let _ = tx.send(msg);
                    }
                    Err(e) => {
                        warn!("UDP receiver error: {}", e);
                        // Brief back-off to avoid spinning on a broken socket
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }
            }
        });

        Ok(())
    }
}

/// Parse key=value pairs from a UDP payload string.
///
/// Loxone UDP messages typically look like:
/// ```text
/// WeatherStation: Temp=23.5 Humidity=65 Wind=12.3
/// ```
/// or simply `Temp=23.5 Humidity=65`.
///
/// Returns a tuple of `(prefix, Vec<(key, value)>)`.
pub fn parse_udp_payload(payload: &str) -> (Option<String>, Vec<(String, String)>) {
    let payload = payload.trim();

    // Check for "prefix: key=value …" pattern
    let (prefix, kv_part) = if let Some(colon) = payload.find(": ") {
        let p = payload[..colon].to_string();
        let rest = &payload[colon + 2..];
        (Some(p), rest)
    } else {
        (None, payload)
    };

    let pairs = kv_part
        .split_whitespace()
        .filter_map(|token| {
            let mut parts = token.splitn(2, '=');
            let key = parts.next()?.to_string();
            let val = parts.next()?.to_string();
            Some((key, val))
        })
        .collect();

    (prefix, pairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_prefix() {
        let (prefix, pairs) = parse_udp_payload("Weather: Temp=23.5 Humidity=65");
        assert_eq!(prefix, Some("Weather".to_string()));
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("Temp".to_string(), "23.5".to_string()));
        assert_eq!(pairs[1], ("Humidity".to_string(), "65".to_string()));
    }

    #[test]
    fn test_parse_without_prefix() {
        let (prefix, pairs) = parse_udp_payload("V1=100 V2=0");
        assert_eq!(prefix, None);
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn test_parse_empty() {
        let (prefix, pairs) = parse_udp_payload("  ");
        assert_eq!(prefix, None);
        assert!(pairs.is_empty());
    }
}
