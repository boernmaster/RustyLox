//! Email configuration management

use rustylox_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Email notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// Enable email notifications
    pub enabled: bool,
    /// SMTP server hostname
    pub smtp_host: String,
    /// SMTP server port (587 for STARTTLS, 465 for SSL)
    pub smtp_port: u16,
    /// SMTP username
    pub smtp_user: String,
    /// SMTP password (stored as plaintext for now; encrypt in production)
    pub smtp_pass: String,
    /// Use TLS/STARTTLS
    pub smtp_tls: bool,
    /// From email address
    pub from_address: String,
    /// From display name
    pub from_name: String,
    /// Recipient addresses for notifications
    pub notification_addresses: Vec<String>,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            smtp_host: String::new(),
            smtp_port: 587,
            smtp_user: String::new(),
            smtp_pass: String::new(),
            smtp_tls: true,
            from_address: String::new(),
            from_name: "RustyLox".to_string(),
            notification_addresses: Vec::new(),
        }
    }
}

/// Manages loading and saving email configuration
pub struct EmailConfigManager {
    config_path: PathBuf,
}

impl EmailConfigManager {
    pub fn new(lbhomedir: &Path) -> Self {
        Self {
            config_path: lbhomedir.join("config/system/email.json"),
        }
    }

    /// Load email config from disk (returns default if file not found)
    pub async fn load(&self) -> Result<EmailConfig> {
        if !self.config_path.exists() {
            return Ok(EmailConfig::default());
        }

        let content = tokio::fs::read_to_string(&self.config_path)
            .await
            .map_err(|e| Error::config(format!("Failed to read email config: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| Error::config(format!("Failed to parse email config: {}", e)))
    }

    /// Save email config to disk
    pub async fn save(&self, config: &EmailConfig) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::config(format!("Failed to create config dir: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(config)
            .map_err(|e| Error::config(format!("Failed to serialize email config: {}", e)))?;

        tokio::fs::write(&self.config_path, content)
            .await
            .map_err(|e| Error::config(format!("Failed to write email config: {}", e)))?;

        Ok(())
    }
}
