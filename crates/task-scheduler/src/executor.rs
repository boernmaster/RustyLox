//! Task execution engine

use crate::config::{ScheduledTask, TaskType};
use chrono::{DateTime, Local, Utc};
use rustylox_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
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

/// Miniserver filesystem directories included in a full backup
const MS_BACKUP_DIRS: &[&str] = &[
    "log", "prog", "sys", "stats", "temp", "update", "web", "user",
];

/// Executes scheduled tasks
pub struct TaskExecutor {
    lbhomedir: PathBuf,
    version: String,
}

impl TaskExecutor {
    pub fn new(lbhomedir: impl Into<PathBuf>, version: impl Into<String>) -> Self {
        Self {
            lbhomedir: lbhomedir.into(),
            version: version.into(),
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
            TaskType::MiniserverBackup => self.run_miniserver_backup().await,
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

    /// Run a system backup using the backup-manager
    async fn run_backup(&self) -> Result<String> {
        info!("Running scheduled system backup");
        let manager =
            backup_manager::BackupManager::new(self.lbhomedir.clone(), self.version.clone());
        let backup_path = manager.create_backup(true).await?;
        let size = tokio::fs::metadata(&backup_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        let name = backup_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| backup_path.display().to_string());
        Ok(format!(
            "System backup created: {} ({} bytes). Use the Backup page to manage backups.",
            name, size
        ))
    }

    /// Rotate log files.
    ///
    /// Rules enforced:
    /// - Active log file (`rustylox.log`) is truncated to 5 MB when it exceeds 10 MB,
    ///   keeping the most recent (tail) content.
    /// - Other `.log` files older than 30 days are deleted.
    async fn run_log_rotation(&self) -> Result<String> {
        info!("Running log rotation");

        const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024; // 10 MB
        const KEEP_BYTES: u64 = 5 * 1024 * 1024; // keep 5 MB of tail after truncation

        let log_dir = self.lbhomedir.join("log");
        if !log_dir.exists() {
            return Ok("Log directory not found, nothing to rotate.".to_string());
        }

        let mut removed = 0u32;
        let mut truncated = 0u32;
        let mut errors = 0u32;
        let cutoff = Utc::now() - chrono::Duration::days(30);

        // Walk both log/ and log/system/ so we catch all log files.
        let dirs: Vec<PathBuf> = vec![log_dir.clone(), log_dir.join("system")];

        for dir in &dirs {
            let mut entries = match tokio::fs::read_dir(dir).await {
                Ok(e) => e,
                Err(_) => continue,
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let is_log = ext == "log" || name.contains(".log");
                if !is_log {
                    continue;
                }

                let metadata = match tokio::fs::metadata(&path).await {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let is_active = name == "rustylox.log";

                if is_active {
                    // Never delete the active log — only truncate if too large.
                    if metadata.len() > MAX_LOG_BYTES {
                        match truncate_log_tail(&path, KEEP_BYTES).await {
                            Ok(()) => {
                                info!(
                                    "Truncated {} to last {} MB",
                                    path.display(),
                                    KEEP_BYTES / (1024 * 1024)
                                );
                                truncated += 1;
                            }
                            Err(e) => {
                                warn!("Failed to truncate {}: {}", path.display(), e);
                                errors += 1;
                            }
                        }
                    }
                } else {
                    // Rotated / old log files: delete when older than 30 days.
                    if let Ok(modified) = metadata.modified() {
                        let modified: DateTime<Utc> = modified.into();
                        if modified < cutoff {
                            if let Err(e) = tokio::fs::remove_file(&path).await {
                                warn!("Failed to remove old log {}: {}", path.display(), e);
                                errors += 1;
                            } else {
                                info!("Removed old log file: {}", path.display());
                                removed += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(format!(
            "Log rotation complete: {} old file(s) removed, {} file(s) truncated, {} error(s).",
            removed, truncated, errors
        ))
    }

    /// Run a system health check
    async fn run_health_check(&self) -> Result<String> {
        info!("Running scheduled health check");

        // Check disk space
        let disk_info = check_disk_space(&self.lbhomedir).await;

        Ok(format!("Health check completed. {}", disk_info))
    }

    /// Run backups for all configured Miniservers
    async fn run_miniserver_backup(&self) -> Result<String> {
        info!("Running scheduled Miniserver backup");

        // Load general config to get Miniserver list
        let config_dir = self.lbhomedir.join("config/system");
        let config_manager = rustylox_config::ConfigManager::new(&config_dir);
        let config = config_manager
            .load_general()
            .await
            .map_err(|e| Error::plugin(format!("Failed to load config: {}", e)))?;

        if config.miniserver.is_empty() {
            return Ok("No Miniservers configured — nothing to back up.".to_string());
        }

        let base_dir = self.lbhomedir.join("data/system/miniserver-backups");
        let mut results: Vec<String> = Vec::new();

        for (id, ms_config) in &config.miniserver {
            if ms_config.ipaddress.is_empty() {
                info!("Miniserver '{}' has no IP — skipping", ms_config.name);
                results.push(format!("'{}': no IP configured, skipped", ms_config.name));
                continue;
            }

            match self.backup_one_miniserver(id, ms_config, &base_dir).await {
                Ok(msg) => {
                    info!("Miniserver '{}' backup OK: {}", ms_config.name, msg);
                    results.push(format!("'{}': {}", ms_config.name, msg));
                }
                Err(e) => {
                    error!("Miniserver '{}' backup failed: {}", ms_config.name, e);
                    results.push(format!("'{}': FAILED — {}", ms_config.name, e));
                }
            }
        }

        Ok(results.join("\n"))
    }

    /// Back up a single Miniserver and return a short summary.
    async fn backup_one_miniserver(
        &self,
        id: &str,
        ms_config: &rustylox_config::MiniserverConfig,
        base_dir: &std::path::Path,
    ) -> Result<String> {
        let client = miniserver_client::MiniserverClient::new(ms_config.clone())
            .map_err(|e| Error::plugin(format!("Cannot create client: {}", e)))?;

        // Walk all backup directories
        let mut all_files: Vec<String> = Vec::new();
        for dir in MS_BACKUP_DIRS {
            let dir_path = format!("/{}/", dir);
            let files = walk_ms_dir(&client, &dir_path).await;
            all_files.extend(files);
        }

        if all_files.is_empty() {
            return Err(Error::plugin("No files found on Miniserver"));
        }

        // Build ZIP in memory
        let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
        let safe_name: String = ms_config
            .name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let filename = format!("Backup_{}_{}.zip", safe_name, timestamp);

        let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
        let mut downloaded = 0usize;
        {
            let mut zip = zip::ZipWriter::new(&mut cursor);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            for file in &all_files {
                let download_url = format!("/dev/fsget{}", file);
                let zip_entry = file.trim_start_matches('/');
                match client.http().download_bytes(&download_url).await {
                    Ok((bytes, _)) => {
                        if zip.start_file(zip_entry, options).is_ok()
                            && zip.write_all(&bytes).is_ok()
                        {
                            downloaded += 1;
                        }
                    }
                    Err(e) => {
                        warn!("Skipping '{}' in Miniserver backup: {}", file, e);
                    }
                }
            }

            zip.finish()
                .map_err(|e| Error::plugin(format!("Failed to finalise ZIP: {}", e)))?;
        }

        if downloaded == 0 {
            return Err(Error::plugin("No files could be downloaded"));
        }

        let zip_bytes = cursor.into_inner();

        // Save to disk
        let dir = ms_backup_dir(base_dir, id, &ms_config.name);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| Error::plugin(format!("Failed to create backup dir: {}", e)))?;

        let backup_path = dir.join(&filename);
        tokio::fs::write(&backup_path, &zip_bytes)
            .await
            .map_err(|e| Error::plugin(format!("Failed to write backup: {}", e)))?;

        // Rotate old backups (keep last 7)
        rotate_ms_backups(&dir, 7).await;

        Ok(format!(
            "{} files → {} ({} bytes)",
            downloaded,
            filename,
            zip_bytes.len()
        ))
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

/// Truncate a log file to its most-recent `keep_bytes` bytes, aligned to a line boundary.
///
/// The oldest content (the beginning of the file) is discarded so the file never grows beyond
/// the configured limit while still retaining the most recent log lines.
async fn truncate_log_tail(path: &PathBuf, keep_bytes: u64) -> Result<()> {
    let content = tokio::fs::read(path)
        .await
        .map_err(|e| Error::plugin(format!("read failed: {}", e)))?;

    let total = content.len() as u64;
    if total <= keep_bytes {
        return Ok(());
    }

    let start_idx = (total - keep_bytes) as usize;
    let tail = &content[start_idx..];

    // Advance past the first partial line so we start on a clean line boundary.
    let line_start = tail
        .iter()
        .position(|&b| b == b'\n')
        .map(|i| i + 1)
        .unwrap_or(0);
    let trimmed = &tail[line_start..];

    tokio::fs::write(path, trimmed)
        .await
        .map_err(|e| Error::plugin(format!("write failed: {}", e)))?;

    Ok(())
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

/// Recursively list all files under `dir` on the Miniserver via /dev/fslist.
async fn walk_ms_dir(client: &miniserver_client::MiniserverClient, dir: &str) -> Vec<String> {
    let mut all_files = Vec::new();
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(dir.to_string());

    while let Some(dir) = queue.pop_front() {
        let url = format!("/dev/fslist{}", dir);
        let listing = match client.http().call(&url).await {
            Ok((_, _, body)) => body,
            Err(e) => {
                warn!("Failed to list Miniserver dir '{}': {}", dir, e);
                continue;
            }
        };
        for line in listing.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(name) = line.split_whitespace().last() {
                if line.starts_with("d ") {
                    queue.push_back(format!("{}{}/", dir, name));
                } else {
                    all_files.push(format!("{}{}", dir, name));
                }
            }
        }
    }

    all_files
}

/// Build the per-Miniserver backup directory path.
fn ms_backup_dir(base_dir: &std::path::Path, id: &str, name: &str) -> PathBuf {
    let safe_name: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    base_dir.join(format!("{}-{}", id, safe_name))
}

/// Delete oldest backups keeping only `keep` most recent ZIP files.
async fn rotate_ms_backups(dir: &std::path::Path, keep: usize) {
    let mut files: Vec<PathBuf> = Vec::new();

    let Ok(mut rd) = tokio::fs::read_dir(dir).await else {
        return;
    };
    while let Ok(Some(entry)) = rd.next_entry().await {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if matches!(ext, "loxone" | "zip") {
            files.push(path);
        }
    }

    if files.len() <= keep {
        return;
    }

    files.sort();
    let to_delete = files.len() - keep;
    for path in files.iter().take(to_delete) {
        if let Err(e) = tokio::fs::remove_file(path).await {
            warn!("Failed to rotate old MS backup {:?}: {}", path, e);
        }
    }
}
