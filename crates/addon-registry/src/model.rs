use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// What an addon POSTs to /api/addons/register.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub version: String,
    pub config_api_base_url: String,
}

impl RegisterRequest {
    /// Registration is unauthenticated and both fields end up in the admin
    /// UI (the name as a URL path segment, the base URL as a link/iframe
    /// target and as the proxy destination), so only plain values pass.
    pub fn validate(&self) -> Result<(), String> {
        let name_ok = !self.name.is_empty()
            && self.name.len() <= 64
            && self
                .name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
            && !self.name.starts_with('.');
        if !name_ok {
            return Err("name must be 1-64 characters of a-z, A-Z, 0-9, '-', '_' or '.'".into());
        }
        let url = reqwest::Url::parse(self.config_api_base_url.trim())
            .map_err(|e| format!("config_api_base_url is not a valid URL: {e}"))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err("config_api_base_url must be an http(s) URL".into());
        }
        Ok(())
    }
}

/// Stored server-side, adds the last-seen timestamp used for staleness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonInstance {
    pub name: String,
    pub version: String,
    pub config_api_base_url: String,
    pub last_seen: DateTime<Utc>,
}

impl AddonInstance {
    pub fn from_request(request: RegisterRequest, now: DateTime<Utc>) -> Self {
        Self {
            name: request.name,
            version: request.version,
            config_api_base_url: request.config_api_base_url,
            last_seen: now,
        }
    }
}

/// What GET /api/addons returns per instance - never includes raw internals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonInstanceView {
    pub name: String,
    pub version: String,
    pub config_api_base_url: String,
    pub online: bool,
}
