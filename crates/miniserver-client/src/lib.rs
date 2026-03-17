//! Miniserver Client - Communication with Loxone Miniservers
//!
//! This crate provides clients for communicating with Loxone Miniservers via:
//! - HTTP/HTTPS (primary protocol)
//! - UDP (lightweight updates)
//! - MQTT (pub/sub messaging)

pub mod delta_cache;
pub mod http;
pub mod reboot_detector;
pub mod udp;
pub mod udp_receiver;

pub use delta_cache::DeltaCache;
pub use http::{MiniserverHttpClient, MonitorCallback, MonitorEvent};
pub use reboot_detector::RebootDetector;
pub use udp::MiniserverUdpClient;
pub use udp_receiver::{MiniserverUdpReceiver, UdpMessage, parse_udp_payload};

use loxberry_config::MiniserverConfig;
use loxberry_core::{Error, Result};
use std::collections::HashMap;

/// Miniserver client combining all protocols
#[derive(Debug)]
pub struct MiniserverClient {
    config: MiniserverConfig,
    http_client: MiniserverHttpClient,
    udp_client: Option<MiniserverUdpClient>,
}

impl MiniserverClient {
    /// Create a new Miniserver client
    pub fn new(config: MiniserverConfig) -> Result<Self> {
        let http_client = MiniserverHttpClient::new(config.clone())?;

        Ok(Self {
            config,
            http_client,
            udp_client: None,
        })
    }

    /// Get the Miniserver configuration
    pub fn config(&self) -> &MiniserverConfig {
        &self.config
    }

    /// Get the HTTP client
    pub fn http(&self) -> &MiniserverHttpClient {
        &self.http_client
    }

    /// Get mutable HTTP client
    pub fn http_mut(&mut self) -> &mut MiniserverHttpClient {
        &mut self.http_client
    }

    /// Initialize UDP client
    pub async fn init_udp(&mut self) -> Result<()> {
        let udp_client = MiniserverUdpClient::new(self.config.clone()).await?;
        self.udp_client = Some(udp_client);
        Ok(())
    }

    /// Get the UDP client (if initialized)
    pub fn udp(&self) -> Option<&MiniserverUdpClient> {
        self.udp_client.as_ref()
    }

    /// Get mutable UDP client (if initialized)
    pub fn udp_mut(&mut self) -> Option<&mut MiniserverUdpClient> {
        self.udp_client.as_mut()
    }

    /// Send HTTP command (mshttp_send equivalent)
    pub async fn send(&self, params: Vec<(String, String)>) -> Result<HashMap<String, bool>> {
        self.http_client.send(params).await
    }

    /// Send HTTP command with delta optimization (mshttp_send_mem equivalent)
    pub async fn send_with_memory(&mut self, params: Vec<(String, String)>) -> Result<bool> {
        self.http_client.send_with_memory(params).await
    }

    /// Get values via HTTP (mshttp_get equivalent)
    pub async fn get(&self, params: Vec<String>) -> Result<HashMap<String, Option<String>>> {
        self.http_client.get(params).await
    }

    /// Send UDP message (msudp_send equivalent)
    pub async fn udp_send(
        &self,
        port: u16,
        prefix: Option<String>,
        params: Vec<(String, String)>,
    ) -> Result<()> {
        if let Some(udp) = &self.udp_client {
            udp.send(port, prefix, params).await
        } else {
            Err(Error::miniserver("UDP client not initialized"))
        }
    }

    /// Send UDP message with delta optimization (msudp_send_mem equivalent)
    pub async fn udp_send_with_memory(
        &mut self,
        port: u16,
        prefix: Option<String>,
        params: Vec<(String, String)>,
    ) -> Result<()> {
        if let Some(udp) = &mut self.udp_client {
            udp.send_with_memory(port, prefix, params).await
        } else {
            Err(Error::miniserver("UDP client not initialized"))
        }
    }
}
