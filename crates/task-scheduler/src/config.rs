//! Task scheduler configuration

use chrono::{DateTime, Utc};
use rustylox_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Type of scheduled task
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// Create a system backup
    Backup,
    /// Rotate log files (remove old entries, compress)
    LogRotation,
    /// Perform a system health check
    HealthCheck,
    /// Execute a custom script
    Custom,
    /// Back up all configured Miniservers
    MiniserverBackup,
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskType::Backup => write!(f, "Backup"),
            TaskType::LogRotation => write!(f, "Log Rotation"),
            TaskType::HealthCheck => write!(f, "Health Check"),
            TaskType::Custom => write!(f, "Custom Script"),
            TaskType::MiniserverBackup => write!(f, "Miniserver Backup"),
        }
    }
}

/// A single scheduled task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    /// Unique task identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Cron expression (e.g., "0 2 * * *" = daily at 2am)
    pub schedule: String,
    /// Task type
    pub task_type: TaskType,
    /// Whether this task is active
    pub enabled: bool,
    /// Script path for Custom task type (relative to LBHOMEDIR)
    pub script_path: Option<String>,
    /// Last execution time
    pub last_run: Option<DateTime<Utc>>,
    /// Last execution status
    pub last_status: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl ScheduledTask {
    /// Create a new task with a random ID
    pub fn new(name: impl Into<String>, schedule: impl Into<String>, task_type: TaskType) -> Self {
        // Generate a simple ID from name + timestamp
        let name = name.into();
        let id = format!(
            "{}_{}",
            name.to_lowercase().replace(' ', "_"),
            Utc::now().timestamp()
        );

        Self {
            id,
            name,
            schedule: schedule.into(),
            task_type,
            enabled: true,
            script_path: None,
            last_run: None,
            last_status: None,
            created_at: Utc::now(),
        }
    }

    /// Calculate the next run time from now
    pub fn next_run(&self) -> Option<DateTime<Utc>> {
        use cron::Schedule;
        use std::str::FromStr;

        let schedule = Schedule::from_str(&self.schedule).ok()?;
        schedule.upcoming(chrono::Utc).next()
    }

    /// Validate the cron expression
    pub fn is_valid_schedule(schedule: &str) -> bool {
        use cron::Schedule;
        use std::str::FromStr;
        Schedule::from_str(schedule).is_ok()
    }
}

/// All scheduled tasks configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScheduledTasksConfig {
    pub tasks: Vec<ScheduledTask>,
}

impl ScheduledTasksConfig {
    /// Default built-in tasks
    pub fn default_tasks() -> Vec<ScheduledTask> {
        vec![
            {
                let mut t = ScheduledTask::new(
                    "Daily Backup",
                    "0 0 2 * * *", // 2am daily (cron crate uses seconds-first format)
                    TaskType::Backup,
                );
                t.id = "builtin_backup".to_string();
                t.enabled = false; // Disabled by default
                t
            },
            {
                let mut t = ScheduledTask::new(
                    "Log Rotation",
                    "0 0 0 * * 0", // Midnight every Sunday
                    TaskType::LogRotation,
                );
                t.id = "builtin_log_rotation".to_string();
                t.enabled = false;
                t
            },
            {
                let mut t = ScheduledTask::new(
                    "Health Check",
                    "0 0 */6 * * *", // Every 6 hours
                    TaskType::HealthCheck,
                );
                t.id = "builtin_health_check".to_string();
                t.enabled = true; // Enabled by default
                t
            },
            {
                let mut t = ScheduledTask::new(
                    "Miniserver Backup",
                    "0 0 3 * * *", // 3am daily
                    TaskType::MiniserverBackup,
                );
                t.id = "builtin_miniserver_backup".to_string();
                t.enabled = false; // Disabled by default
                t
            },
        ]
    }
}

/// Manages loading and saving scheduled tasks configuration
pub struct ScheduledTasksConfigManager {
    config_path: PathBuf,
}

impl ScheduledTasksConfigManager {
    pub fn new(lbhomedir: &Path) -> Self {
        Self {
            config_path: lbhomedir.join("config/system/scheduled_tasks.json"),
        }
    }

    /// Load config from disk (returns defaults if not found).
    /// Merges any built-in tasks that are missing from an existing config so
    /// that newly-added defaults (e.g. MiniserverBackup) appear after upgrades.
    pub async fn load(&self) -> Result<ScheduledTasksConfig> {
        if !self.config_path.exists() {
            return Ok(ScheduledTasksConfig {
                tasks: ScheduledTasksConfig::default_tasks(),
            });
        }

        let content = tokio::fs::read_to_string(&self.config_path)
            .await
            .map_err(|e| Error::config(format!("Failed to read tasks config: {}", e)))?;

        let mut config: ScheduledTasksConfig = serde_json::from_str(&content)
            .map_err(|e| Error::config(format!("Failed to parse tasks config: {}", e)))?;

        // Merge built-in tasks that are absent from the on-disk config so that
        // newly-added defaults become visible without wiping user data.
        for default_task in ScheduledTasksConfig::default_tasks() {
            if !config.tasks.iter().any(|t| t.id == default_task.id) {
                config.tasks.push(default_task);
            }
        }

        Ok(config)
    }

    /// Save config to disk
    pub async fn save(&self, config: &ScheduledTasksConfig) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::config(format!("Failed to create config dir: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(config)
            .map_err(|e| Error::config(format!("Failed to serialize tasks config: {}", e)))?;

        tokio::fs::write(&self.config_path, content)
            .await
            .map_err(|e| Error::config(format!("Failed to write tasks config: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_cron_schedule() {
        assert!(ScheduledTask::is_valid_schedule("0 0 2 * * *"));
        assert!(ScheduledTask::is_valid_schedule("0 0 * * * *"));
        assert!(!ScheduledTask::is_valid_schedule("invalid"));
        assert!(!ScheduledTask::is_valid_schedule("* * *")); // too few fields
    }

    #[test]
    fn test_next_run() {
        let task = ScheduledTask::new("Test", "0 0 * * * *", TaskType::HealthCheck);
        let next = task.next_run();
        assert!(next.is_some());
        assert!(next.unwrap() > Utc::now());
    }

    #[test]
    fn test_default_tasks() {
        let tasks = ScheduledTasksConfig::default_tasks();
        assert_eq!(tasks.len(), 4);
        assert!(tasks.iter().any(|t| t.task_type == TaskType::Backup));
        assert!(tasks.iter().any(|t| t.task_type == TaskType::LogRotation));
        assert!(tasks.iter().any(|t| t.task_type == TaskType::HealthCheck));
        assert!(tasks
            .iter()
            .any(|t| t.task_type == TaskType::MiniserverBackup));
    }
}
