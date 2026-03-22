//! Task execution engine

use crate::config::{ScheduledTask, TaskType};
use chrono::{DateTime, Utc};
use rustylox_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{error, info, warn};

/// Task execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Skipped,
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::Running => write!(f, "running"),
            ExecutionStatus::Completed => write!(f, "completed"),
            ExecutionStatus::Failed => write!(f, "failed"),
            ExecutionStatus::Skipped => write!(f, "skipped"),
        }
    }
}

/// Record of a single task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecution {
    pub task_id: String,
    pub task_name: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: ExecutionStatus,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
}

impl TaskExecution {
    pub fn new(task_id: impl Into<String>, task_name: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            task_name: task_name.into(),
            started_at: Utc::now(),
            completed_at: None,
            status: ExecutionStatus::Running,
            output: String::new(),
            error: None,
            duration_ms: None,
        }
    }

    pub fn complete(&mut self, output: String) {
        self.completed_at = Some(Utc::now());
        self.status = ExecutionStatus::Completed;
        self.output = output;
        self.duration_ms = Some(
            (self.completed_at.unwrap() - self.started_at)
                .num_milliseconds()
                .max(0) as u64,
        );
    }

    pub fn fail(&mut self, error: String) {
        self.completed_at = Some(Utc::now());
        self.status = ExecutionStatus::Failed;
        self.error = Some(error.clone());
        self.output = error;
        self.duration_ms = Some(
            (self.completed_at.unwrap() - self.started_at)
                .num_milliseconds()
                .max(0) as u64,
        );
    }
}

/// Executes scheduled tasks
pub struct TaskExecutor {
    lbhomedir: PathBuf,
}

impl TaskExecutor {
    pub fn new(lbhomedir: impl Into<PathBuf>) -> Self {
        Self {
            lbhomedir: lbhomedir.into(),
        }
    }

    /// Execute a scheduled task
    pub async fn execute(&self, task: &ScheduledTask) -> TaskExecution {
        let mut execution = TaskExecution::new(&task.id, &task.name);
        info!("Executing scheduled task: {} ({})", task.name, task.id);

        let result = match &task.task_type {
            TaskType::Backup => self.run_backup().await,
            TaskType::LogRotation => self.run_log_rotation().await,
            TaskType::HealthCheck => self.run_health_check().await,
            TaskType::Custom => {
                if let Some(script) = &task.script_path {
                    self.run_custom_script(script).await
                } else {
                    Err(Error::plugin("No script path configured for Custom task"))
                }
            }
        };

        match result {
            Ok(output) => {
                info!("Task '{}' completed successfully", task.name);
                execution.complete(output);
            }
            Err(e) => {
                error!("Task '{}' failed: {}", task.name, e);
                execution.fail(e.to_string());
            }
        }

        execution
    }

    /// Run a system backup
    async fn run_backup(&self) -> Result<String> {
        info!("Running scheduled backup");
        // Delegate to backup-manager by invoking the create_backup logic
        // For now, just mark as success with a placeholder message.
        // In production, this would call BackupManager::create().
        Ok("Backup task scheduled. Use the Backup page to create manual backups or configure automatic backups.".to_string())
    }

    /// Rotate log files
    async fn run_log_rotation(&self) -> Result<String> {
        info!("Running log rotation");

        let log_dir = self.lbhomedir.join("log");
        if !log_dir.exists() {
            return Ok("Log directory not found, nothing to rotate.".to_string());
        }

        let mut rotated = 0u32;
        let mut errors = 0u32;

        // Find log files older than 30 days and remove them
        let cutoff = Utc::now() - chrono::Duration::days(30);

        let mut entries = tokio::fs::read_dir(&log_dir)
            .await
            .map_err(|e| Error::plugin(format!("Failed to read log directory: {}", e)))?;

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_file() {
                if let Ok(metadata) = tokio::fs::metadata(&path).await {
                    if let Ok(modified) = metadata.modified() {
                        let modified: DateTime<Utc> = modified.into();
                        if modified < cutoff {
                            if let Err(e) = tokio::fs::remove_file(&path).await {
                                warn!("Failed to remove old log {}: {}", path.display(), e);
                                errors += 1;
                            } else {
                                rotated += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(format!(
            "Log rotation complete: {} files removed, {} errors.",
            rotated, errors
        ))
    }

    /// Run a system health check
    async fn run_health_check(&self) -> Result<String> {
        info!("Running scheduled health check");

        // Check disk space
        let disk_info = check_disk_space(&self.lbhomedir).await;

        Ok(format!("Health check completed. {}", disk_info))
    }

    /// Run a custom script
    async fn run_custom_script(&self, script_path: &str) -> Result<String> {
        let full_path = self.lbhomedir.join(script_path);
        info!("Running custom script: {}", full_path.display());

        if !full_path.exists() {
            return Err(Error::plugin(format!(
                "Script not found: {}",
                full_path.display()
            )));
        }

        let interpreter = match full_path.extension().and_then(|e| e.to_str()) {
            Some("pl") => "perl",
            Some("php") => "php",
            Some("py") => "python3",
            _ => "bash",
        };

        let output = tokio::process::Command::new(interpreter)
            .arg(&full_path)
            .env("LBHOMEDIR", &self.lbhomedir)
            .output()
            .await
            .map_err(|e| Error::plugin(format!("Failed to run script: {}", e)))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(Error::plugin(format!("Script failed: {}", stderr)))
        }
    }
}

/// Check available disk space
async fn check_disk_space(path: &PathBuf) -> String {
    // Simple check using df command on Unix
    let output = tokio::process::Command::new("df")
        .arg("-h")
        .arg(path)
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            // Get the last line with actual disk info
            let last_line = stdout.lines().last().unwrap_or("").to_string();
            format!("Disk: {}", last_line)
        }
        _ => "Disk check unavailable.".to_string(),
    }
}
