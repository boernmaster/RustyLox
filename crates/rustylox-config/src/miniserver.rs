//! Miniserver configuration

use serde::{Deserialize, Serialize};

/// Miniserver configuration entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiniserverConfig {
    #[serde(rename = "Admin")]
    pub admin: String,

    #[serde(rename = "Admin_raw")]
    pub admin_raw: String,

    #[serde(rename = "Cloudurl")]
    pub cloudurl: String,

    #[serde(rename = "Cloudurlftpport")]
    pub cloudurlftpport: String,

    #[serde(rename = "Credentials")]
    pub credentials: String,

    #[serde(rename = "Credentials_raw")]
    pub credentials_raw: String,

    #[serde(rename = "Encryptresponse")]
    pub encryptresponse: String,

    #[serde(rename = "Fulluri")]
    pub fulluri: String,

    #[serde(rename = "Fulluri_raw")]
    pub fulluri_raw: String,

    #[serde(rename = "Ipaddress")]
    pub ipaddress: String,

    #[serde(rename = "Ipv6format")]
    pub ipv6format: String,

    #[serde(rename = "Name")]
    pub name: String,

    #[serde(rename = "Note")]
    pub note: String,

    #[serde(rename = "Pass")]
    pub pass: String,

    #[serde(rename = "Pass_raw")]
    pub pass_raw: String,

    #[serde(rename = "Port")]
    pub port: String,

    #[serde(rename = "Porthttps")]
    pub porthttps: Option<String>,

    #[serde(rename = "Preferhttps")]
    pub preferhttps: Option<String>,

    #[serde(rename = "Securegateway")]
    pub securegateway: String,

    #[serde(rename = "Transport")]
    pub transport: String,

    #[serde(rename = "Useclouddns")]
    pub useclouddns: String,

    #[serde(rename = "Udpport")]
    pub udpport: Option<String>,
}

impl MiniserverConfig {
    /// Check if HTTPS is preferred
    pub fn prefers_https(&self) -> bool {
        self.preferhttps.as_deref() == Some("1")
    }

    /// Check if CloudDNS is enabled
    pub fn uses_clouddns(&self) -> bool {
        self.useclouddns == "1"
    }

    /// Get the effective port based on transport
    pub fn effective_port(&self) -> u16 {
        if self.transport == "https" {
            self.porthttps
                .as_deref()
                .and_then(|p| p.parse().ok())
                .unwrap_or(443)
        } else {
            self.port.parse().unwrap_or(80)
        }
    }

    /// Build the base URL without credentials (safe for use with reqwest basic_auth)
    pub fn build_base_url(&self) -> String {
        format!(
            "{}://{}:{}",
            self.transport,
            self.ipaddress,
            self.effective_port()
        )
    }

    /// Build the full URI with credentials embedded in the URL.
    ///
    /// WARNING: Only use this when the credentials are guaranteed to contain no
    /// URL-special characters (e.g. `#`, `&`, `?`). Prefer `build_base_url()`
    /// combined with reqwest's `.basic_auth()` for production HTTP requests.
    pub fn build_uri(&self) -> String {
        if !self.credentials_raw.is_empty() && self.credentials_raw != ":" {
            format!(
                "{}://{}@{}:{}",
                self.transport,
                self.credentials_raw,
                self.ipaddress,
                self.effective_port()
            )
        } else {
            self.build_base_url()
        }
    }
}

impl Default for MiniserverConfig {
    fn default() -> Self {
        Self {
            admin: String::new(),
            admin_raw: String::new(),
            cloudurl: String::new(),
            cloudurlftpport: String::new(),
            credentials: ":".to_string(),
            credentials_raw: ":".to_string(),
            encryptresponse: String::new(),
            fulluri: "http://:@:80".to_string(),
            fulluri_raw: "http://:@:80".to_string(),
            ipaddress: String::new(),
            ipv6format: "0".to_string(),
            name: String::new(),
            note: String::new(),
            pass: String::new(),
            pass_raw: String::new(),
            port: "80".to_string(),
            porthttps: None,
            preferhttps: None,
            securegateway: String::new(),
            transport: "http".to_string(),
            useclouddns: "0".to_string(),
            udpport: None,
        }
    }
}
