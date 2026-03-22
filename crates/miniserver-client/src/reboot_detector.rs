//! Miniserver reboot detection
//!
//! Detects when a Miniserver has rebooted by monitoring the /dev/lan/txp endpoint.

use rustylox_config::MiniserverConfig;
use rustylox_core::{Error, Result};
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, trace};

/// Reboot detector for Miniserver
#[derive(Debug, Clone)]
pub struct RebootDetector {
    config: MiniserverConfig,
    client: Arc<Client>,
    last_check: Arc<Mutex<Option<Instant>>>,
    last_txp_value: Arc<Mutex<Option<String>>>,
}

impl RebootDetector {
    /// Create a new reboot detector
    pub fn new(config: MiniserverConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(1))
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true)
            .build()
            .unwrap();

        Self {
            config,
            client: Arc::new(client),
            last_check: Arc::new(Mutex::new(None)),
            last_txp_value: Arc::new(Mutex::new(None)),
        }
    }

    /// Check if Miniserver has rebooted
    ///
    /// Returns true if a reboot was detected since last check
    pub async fn check_reboot(&self) -> Result<bool> {
        let mut last_check = self.last_check.lock().await;
        let mut last_value = self.last_txp_value.lock().await;

        // Rate limit checks to every 5 minutes
        if let Some(last) = *last_check {
            if last.elapsed() < Duration::from_secs(300) {
                return Ok(false);
            }
        }

        // Query /dev/lan/txp endpoint
        let txp_value = match self.query_txp().await {
            Ok(val) => val,
            Err(e) => {
                debug!("Failed to query txp endpoint: {}", e);
                *last_check = Some(Instant::now());
                return Ok(false);
            }
        };

        trace!("TXP value: {}", txp_value);

        let rebooted = if let Some(prev_value) = last_value.as_ref() {
            // If value decreased or changed significantly, assume reboot
            match (prev_value.parse::<u64>(), txp_value.parse::<u64>()) {
                (Ok(prev), Ok(current)) => {
                    if current < prev {
                        debug!("Miniserver reboot detected (txp: {} -> {})", prev, current);
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            }
        } else {
            // First check, no reboot
            false
        };

        *last_value = Some(txp_value);
        *last_check = Some(Instant::now());

        Ok(rebooted)
    }

    /// Query the /dev/lan/txp endpoint
    async fn query_txp(&self) -> Result<String> {
        let url = format!("{}/dev/lan/txp", self.config.build_base_url());

        let response = self
            .client
            .get(&url)
            .basic_auth(&self.config.admin_raw, Some(&self.config.pass_raw))
            .send()
            .await
            .map_err(|e| Error::network(format!("TXP query failed: {}", e)))?;

        let body = response
            .text()
            .await
            .map_err(|e| Error::network(format!("Failed to read TXP response: {}", e)))?;

        // Extract value from XML
        extract_xml_value(&body).ok_or_else(|| Error::miniserver("Failed to parse TXP response"))
    }

    /// Reset reboot detection state
    pub async fn reset(&self) {
        let mut last_check = self.last_check.lock().await;
        let mut last_value = self.last_txp_value.lock().await;

        *last_check = None;
        *last_value = None;
    }
}

/// Extract value from XML response
fn extract_xml_value(xml: &str) -> Option<String> {
    if let Some(start) = xml.find("value=\"") {
        let start_pos = start + 7; // length of "value=\""
        if let Some(end) = xml[start_pos..].find('"') {
            return Some(xml[start_pos..start_pos + end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_xml_value() {
        let xml = r#"<LL value="12345" Code="200"/>"#;
        assert_eq!(extract_xml_value(xml), Some("12345".to_string()));

        let xml_no_value = r#"<LL Code="200"/>"#;
        assert_eq!(extract_xml_value(xml_no_value), None);
    }
}
