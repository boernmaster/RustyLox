//! UDP client for Miniserver communication

use loxberry_config::MiniserverConfig;
use loxberry_core::{Error, Result};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::{debug, trace};

use crate::delta_cache::DeltaCache;

const MAX_UDP_PACKET_SIZE: usize = 220;
const DEFAULT_DELIMITER: &str = "=";

/// UDP client for Miniserver
#[derive(Debug)]
pub struct MiniserverUdpClient {
    #[allow(dead_code)]
    config: MiniserverConfig,
    socket: UdpSocket,
    target_addr: SocketAddr,
    delta_cache: DeltaCache,
    delimiter: String,
}

impl MiniserverUdpClient {
    /// Create a new UDP client
    pub async fn new(config: MiniserverConfig) -> Result<Self> {
        // Bind to any available port
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| Error::network(format!("Failed to bind UDP socket: {}", e)))?;

        // Parse target address
        let target_addr = format!("{}:{}", config.ipaddress, config.effective_port())
            .parse()
            .map_err(|e| Error::config(format!("Invalid Miniserver address: {}", e)))?;

        Ok(Self {
            config,
            socket,
            target_addr,
            delta_cache: DeltaCache::new(),
            delimiter: DEFAULT_DELIMITER.to_string(),
        })
    }

    /// Send UDP message (msudp_send equivalent)
    ///
    /// # Arguments
    /// * `port` - Target port on Miniserver
    /// * `prefix` - Optional prefix for the message
    /// * `params` - Vector of (key, value) tuples
    pub async fn send(
        &self,
        port: u16,
        prefix: Option<String>,
        params: Vec<(String, String)>,
    ) -> Result<()> {
        let message = self.build_message(prefix, params);

        // Split into chunks if necessary (max 220 bytes per packet)
        let chunks = self.chunk_message(&message, MAX_UDP_PACKET_SIZE);

        let mut target = self.target_addr;
        target.set_port(port);

        for chunk in chunks {
            debug!("Sending UDP to {}: {}", target, chunk);

            self.socket
                .send_to(chunk.as_bytes(), target)
                .await
                .map_err(|e| Error::network(format!("UDP send failed: {}", e)))?;
        }

        Ok(())
    }

    /// Send UDP message with delta optimization (msudp_send_mem equivalent)
    pub async fn send_with_memory(
        &mut self,
        port: u16,
        prefix: Option<String>,
        params: Vec<(String, String)>,
    ) -> Result<()> {
        // Filter params that have changed
        let changed_params: Vec<_> = params
            .into_iter()
            .filter(|(k, v)| self.delta_cache.has_changed(k, v))
            .collect();

        if changed_params.is_empty() {
            trace!("No parameters changed, skipping UDP send");
            return Ok(());
        }

        // Send changed parameters
        self.send(port, prefix.clone(), changed_params.clone())
            .await?;

        // Update cache
        for (key, value) in changed_params {
            self.delta_cache.update(&key, value);
        }

        Ok(())
    }

    /// Build UDP message from parameters
    ///
    /// Format: "prefix: key=value key2=value2"
    fn build_message(&self, prefix: Option<String>, params: Vec<(String, String)>) -> String {
        let param_str: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{}{}{}", k, self.delimiter, v))
            .collect();

        if let Some(prefix) = prefix {
            format!("{}: {}", prefix, param_str.join(" "))
        } else {
            param_str.join(" ")
        }
    }

    /// Split message into chunks of maximum size
    fn chunk_message(&self, message: &str, max_size: usize) -> Vec<String> {
        if message.len() <= max_size {
            return vec![message.to_string()];
        }

        let mut chunks = Vec::new();
        let words: Vec<&str> = message.split(' ').collect();
        let mut current_chunk = String::new();

        for word in words {
            if current_chunk.len() + word.len() + 1 > max_size {
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk);
                    current_chunk = String::new();
                }

                // If single word is too long, truncate it
                if word.len() > max_size {
                    chunks.push(word[..max_size].to_string());
                    continue;
                }
            }

            if !current_chunk.is_empty() {
                current_chunk.push(' ');
            }
            current_chunk.push_str(word);
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }

    /// Set custom delimiter (default is "=")
    pub fn set_delimiter(&mut self, delimiter: String) {
        self.delimiter = delimiter;
    }

    /// Clear the delta cache
    pub fn clear_cache(&mut self) {
        self.delta_cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn make_test_client() -> MiniserverUdpClient {
        let config = MiniserverConfig::default();
        let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        MiniserverUdpClient {
            config,
            socket,
            target_addr: "127.0.0.1:8080".parse().unwrap(),
            delta_cache: DeltaCache::new(),
            delimiter: "=".to_string(),
        }
    }

    #[tokio::test]
    async fn test_build_message() {
        let client = make_test_client().await;

        let message = client.build_message(
            Some("Weather".to_string()),
            vec![
                ("Temp".to_string(), "23.5".to_string()),
                ("Humidity".to_string(), "65".to_string()),
            ],
        );

        assert_eq!(message, "Weather: Temp=23.5 Humidity=65");
    }

    #[tokio::test]
    async fn test_chunk_message() {
        let client = make_test_client().await;

        let short_message = "test message";
        let chunks = client.chunk_message(short_message, 220);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], short_message);

        // Test chunking
        let long_message = "a ".repeat(200); // 400 characters
        let chunks = client.chunk_message(&long_message, 220);
        assert!(chunks.len() > 1);
        assert!(chunks[0].len() <= 220);
    }
}
