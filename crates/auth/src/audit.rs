//! Audit logging - records sensitive security-relevant actions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::warn;

/// Auditable actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Login,
    LoginFailed,
    Logout,
    CreateUser,
    UpdateUser,
    DeleteUser,
    CreateApiKey,
    DeleteApiKey,
    UpdateConfig,
    InstallPlugin,
    UninstallPlugin,
    CreateBackup,
    RestoreBackup,
    SendCommand,
    AccessDenied,
    PasswordChanged,
    AccountLocked,
    AccountUnlocked,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).unwrap_or_else(|_| "unknown".into());
        write!(f, "{}", s.trim_matches('"'))
    }
}

/// Single audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub user: String,
    pub action: AuditAction,
    pub resource: String,
    pub ip_address: String,
    pub success: bool,
    pub details: Option<String>,
}

/// Append-only audit logger
#[derive(Debug, Clone)]
pub struct AuditLogger {
    log_path: PathBuf,
}

impl AuditLogger {
    pub fn new(log_dir: &Path) -> Self {
        Self {
            log_path: log_dir.join("audit.log"),
        }
    }

    /// Write a single audit entry (fire and forget - logs warning on failure)
    pub async fn log(
        &self,
        user: impl Into<String>,
        action: AuditAction,
        resource: impl Into<String>,
        ip_address: impl Into<String>,
        success: bool,
        details: Option<String>,
    ) {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            user: user.into(),
            action,
            resource: resource.into(),
            ip_address: ip_address.into(),
            success,
            details,
        };

        if let Err(e) = self.write_entry(&entry).await {
            warn!("Failed to write audit log entry: {}", e);
        }
    }

    async fn write_entry(&self, entry: &AuditEntry) -> std::io::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.log_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut line = serde_json::to_string(entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        line.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await?;

        file.write_all(line.as_bytes()).await?;
        Ok(())
    }

    /// Read recent audit log entries (last N lines)
    pub async fn read_recent(&self, limit: usize) -> Vec<AuditEntry> {
        let Ok(content) = tokio::fs::read_to_string(&self.log_path).await else {
            return vec![];
        };

        let mut entries: Vec<AuditEntry> = content
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();

        let total = entries.len();
        if total > limit {
            entries.drain(0..total - limit);
        }
        entries
    }
}
