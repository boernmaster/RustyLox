//! LoxBerry Configuration - JSON config file management
//!
//! This crate handles reading and writing LoxBerry configuration files.

pub mod general;
pub mod miniserver;
pub mod mqtt;

pub use general::GeneralConfig;
pub use miniserver::MiniserverConfig;
pub use mqtt::MqttConfig;

use loxberry_core::{Error, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Configuration manager for LoxBerry
#[derive(Debug, Clone)]
pub struct ConfigManager {
    config_dir: PathBuf,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(config_dir: impl Into<PathBuf>) -> Self {
        Self {
            config_dir: config_dir.into(),
        }
    }

    /// Get the path to general.json
    pub fn general_json_path(&self) -> PathBuf {
        self.config_dir.join("general.json")
    }

    /// Load general configuration
    pub async fn load_general(&self) -> Result<GeneralConfig> {
        let path = self.general_json_path();
        let content = fs::read_to_string(&path).await.map_err(|e| {
            Error::config(format!("Failed to read {}: {}", path.display(), e))
        })?;

        serde_json::from_str(&content).map_err(|e| {
            Error::config(format!("Failed to parse {}: {}", path.display(), e))
        })
    }

    /// Save general configuration
    pub async fn save_general(&self, config: &GeneralConfig) -> Result<()> {
        let path = self.general_json_path();
        let content = serde_json::to_string_pretty(config)?;

        fs::write(&path, content).await.map_err(|e| {
            Error::config(format!("Failed to write {}: {}", path.display(), e))
        })
    }
}
