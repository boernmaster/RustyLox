use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// What an addon POSTs to /api/addons/register.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub version: String,
    pub config_api_base_url: String,
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
