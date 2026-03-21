//! HTTP/HTTPS client for Miniserver communication

use loxberry_config::MiniserverConfig;
use loxberry_core::{Error, Result};
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, trace};
use urlencoding::encode;

use crate::delta_cache::DeltaCache;
use crate::reboot_detector::RebootDetector;

/// Event callback for monitoring Miniserver communication
pub type MonitorCallback = Arc<dyn Fn(MonitorEvent) + Send + Sync>;

/// Miniserver communication event for monitoring
#[derive(Debug, Clone)]
pub struct MonitorEvent {
    pub direction: String, // "sent", "received", "error"
    pub protocol: String,  // "http", "udp"
    pub url: Option<String>,
    pub params: Option<String>,
    pub response: Option<String>,
    pub code: Option<String>,
    pub error: Option<String>,
}

/// HTTP/HTTPS client for Miniserver
pub struct MiniserverHttpClient {
    config: MiniserverConfig,
    client: Client,
    delta_cache: DeltaCache,
    reboot_detector: RebootDetector,
    monitor_callback: Option<MonitorCallback>,
}

impl std::fmt::Debug for MiniserverHttpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MiniserverHttpClient")
            .field("config", &self.config)
            .field("client", &self.client)
            .field("delta_cache", &self.delta_cache)
            .field("reboot_detector", &self.reboot_detector)
            .field("has_monitor", &self.monitor_callback.is_some())
            .finish()
    }
}

/// Miniserver XML response structure
#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
struct MiniserverResponse {
    #[serde(rename = "@value")]
    value: Option<String>,

    #[serde(rename = "@Code")]
    code: Option<String>,
}

impl MiniserverHttpClient {
    /// Create a new HTTP client for a Miniserver
    pub fn new(config: MiniserverConfig) -> Result<Self> {
        // Build HTTP client with SSL verification disabled (for self-signed certs)
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(5))
            .danger_accept_invalid_certs(true) // Accept self-signed certificates
            .danger_accept_invalid_hostnames(true)
            .build()
            .map_err(|e| Error::network(format!("Failed to create HTTP client: {}", e)))?;

        let delta_cache = DeltaCache::new();
        let reboot_detector = RebootDetector::new(config.clone());

        Ok(Self {
            config,
            client,
            delta_cache,
            reboot_detector,
            monitor_callback: None,
        })
    }

    /// Set monitor callback for tracking communication
    pub fn set_monitor_callback(&mut self, callback: MonitorCallback) {
        self.monitor_callback = Some(callback);
    }

    /// Emit a monitoring event
    fn emit_event(&self, event: MonitorEvent) {
        if let Some(callback) = &self.monitor_callback {
            callback(event);
        }
    }

    /// Send HTTP command to Miniserver (mshttp_send equivalent)
    ///
    /// # Arguments
    /// * `params` - Vector of (parameter, value) tuples
    ///
    /// # Returns
    /// HashMap with parameter names as keys and success status as values
    pub async fn send(&self, params: Vec<(String, String)>) -> Result<HashMap<String, bool>> {
        let mut results = HashMap::new();

        for (param, value) in params {
            let success = self.send_single(&param, &value).await.is_ok();
            results.insert(param, success);
        }

        Ok(results)
    }

    /// Send single parameter/value pair
    async fn send_single(&self, param: &str, value: &str) -> Result<String> {
        let command = format!("/dev/sps/io/{}/{}", encode(param), encode(value));
        let url = self.build_url(&command);

        // Emit sent event
        self.emit_event(MonitorEvent {
            direction: "sent".to_string(),
            protocol: "http".to_string(),
            url: Some(url.clone()),
            params: Some(format!("{}={}", param, value)),
            response: None,
            code: None,
            error: None,
        });

        let result = self.call(&command).await;

        match result {
            Ok((response_value, code, _raw)) => {
                // Emit received event
                self.emit_event(MonitorEvent {
                    direction: "received".to_string(),
                    protocol: "http".to_string(),
                    url: Some(url),
                    params: None,
                    response: response_value.clone(),
                    code: code.clone(),
                    error: None,
                });

                // Check response code
                if let Some(code_str) = &code {
                    if code_str == "200" {
                        Ok(response_value.unwrap_or_default())
                    } else {
                        Err(Error::miniserver(format!(
                            "Miniserver returned error code: {}",
                            code_str
                        )))
                    }
                } else {
                    Ok(response_value.unwrap_or_default())
                }
            }
            Err(e) => {
                // Emit error event
                self.emit_event(MonitorEvent {
                    direction: "error".to_string(),
                    protocol: "http".to_string(),
                    url: Some(url),
                    params: Some(format!("{}={}", param, value)),
                    response: None,
                    code: None,
                    error: Some(e.to_string()),
                });
                Err(e)
            }
        }
    }

    /// Send HTTP command with delta optimization (mshttp_send_mem equivalent)
    ///
    /// Only sends if values have changed from cached version
    pub async fn send_with_memory(&mut self, params: Vec<(String, String)>) -> Result<bool> {
        // Check for Miniserver reboot
        if self.reboot_detector.check_reboot().await? {
            debug!("Miniserver rebooted, clearing delta cache");
            self.delta_cache.clear();
        }

        // Filter params that have changed
        let changed_params: Vec<_> = params
            .into_iter()
            .filter(|(k, v)| self.delta_cache.has_changed(k, v))
            .collect();

        if changed_params.is_empty() {
            trace!("No parameters changed, skipping send");
            return Ok(false);
        }

        // Send changed parameters
        let results = self.send(changed_params.clone()).await?;

        // Update cache for successful sends
        for (param, value) in changed_params {
            if results.get(&param).copied().unwrap_or(false) {
                self.delta_cache.update(&param, value);
            }
        }

        Ok(true)
    }

    /// Get values from Miniserver (mshttp_get equivalent)
    ///
    /// # Arguments
    /// * `params` - Vector of parameter names to retrieve
    ///
    /// # Returns
    /// HashMap with parameter names as keys and values (or None if not found)
    pub async fn get(&self, params: Vec<String>) -> Result<HashMap<String, Option<String>>> {
        let mut results = HashMap::new();

        for param in params {
            let value = self.get_single(&param).await.ok();
            results.insert(param, value);
        }

        Ok(results)
    }

    /// Get single parameter value
    async fn get_single(&self, param: &str) -> Result<String> {
        // Try with /all suffix first
        let command = format!("/dev/sps/io/{}/all", encode(param));
        let result = self.call(&command).await;

        match result {
            Ok((Some(value), _, _)) => {
                // Workaround for analog outputs that always return 0 with /all
                if value == "0" {
                    // Try without /all suffix
                    let command_plain = format!("/dev/sps/io/{}", encode(param));
                    if let Ok((Some(val), _, _)) = self.call(&command_plain).await {
                        if val != "0" {
                            return Ok(val);
                        }
                    }
                }
                Ok(value)
            }
            Ok((None, _, _)) => Err(Error::miniserver("No value in response")),
            Err(_e) => {
                // Fallback to plain endpoint
                let command_plain = format!("/dev/sps/io/{}", encode(param));
                let (value, _, _) = self.call(&command_plain).await?;
                value.ok_or_else(|| Error::miniserver("No value in response"))
            }
        }
    }

    /// Raw HTTP call to Miniserver (mshttp_call equivalent)
    /// Redact credentials from URL for secure logging
    fn redact_url_credentials(url: &str) -> String {
        // Replace user:pass@ with ***:***@
        if let Some(at_pos) = url.find('@') {
            if let Some(proto_end) = url.find("://") {
                let proto_part = &url[..proto_end + 3]; // "http://"
                let after_at = &url[at_pos..]; // "@host:port/path"
                return format!("{}***:***{}", proto_part, after_at);
            }
        }
        url.to_string()
    }

    ///
    /// # Arguments
    /// * `command` - Command path (e.g., "/dev/sps/io/V1/100")
    ///
    /// # Returns
    /// Tuple of (value, code, raw_response)
    pub async fn call(&self, command: &str) -> Result<(Option<String>, Option<String>, String)> {
        let url = self.build_url(command);

        debug!(
            "Miniserver HTTP call: {}",
            Self::redact_url_credentials(&url)
        );

        let response = self
            .client
            .get(&url)
            .basic_auth(&self.config.admin_raw, Some(&self.config.pass_raw))
            .send()
            .await
            .map_err(|e| Error::network(format!("HTTP request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| Error::network(format!("Failed to read response: {}", e)))?;

        trace!("Response ({}): {}", status, body);

        if !status.is_success() {
            return Err(Error::network(format!("HTTP error: {}", status)));
        }

        // Parse XML response
        let (value, code) = self.parse_response(&body)?;

        Ok((value, code, body))
    }

    /// Build URL without credentials (credentials are sent via basic_auth)
    fn build_url(&self, command: &str) -> String {
        format!("{}{}", self.config.build_base_url(), command)
    }

    /// Parse XML response from Miniserver
    fn parse_response(&self, xml: &str) -> Result<(Option<String>, Option<String>)> {
        // Simple XML parsing - look for value and Code attributes
        let value = extract_xml_attr(xml, "value");
        let code = extract_xml_attr(xml, "Code");

        Ok((value, code))
    }

    /// Clear the delta cache
    pub fn clear_cache(&mut self) {
        self.delta_cache.clear();
    }
}

/// Extract XML attribute value (simple implementation)
fn extract_xml_attr(xml: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    if let Some(start) = xml.find(&pattern) {
        let start_pos = start + pattern.len();
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
    fn test_extract_xml_attr() {
        let xml = r#"<LL value="123" Code="200"/>"#;
        assert_eq!(extract_xml_attr(xml, "value"), Some("123".to_string()));
        assert_eq!(extract_xml_attr(xml, "Code"), Some("200".to_string()));
        assert_eq!(extract_xml_attr(xml, "missing"), None);
    }

    #[test]
    fn test_build_url() {
        let config = MiniserverConfig {
            transport: "http".to_string(),
            ipaddress: "192.168.1.100".to_string(),
            port: "80".to_string(),
            credentials_raw: "admin:password".to_string(),
            ..Default::default()
        };

        let client = MiniserverHttpClient::new(config).unwrap();
        let url = client.build_url("/dev/sps/io/V1/100");

        assert!(url.contains("192.168.1.100"));
        assert!(url.contains("/dev/sps/io/V1/100"));
    }
}
