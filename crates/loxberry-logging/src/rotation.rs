//! Log rotation and cleanup

use chrono::{DateTime, Duration, Utc};
use loxberry_core::Result;
use std::path::PathBuf;

/// Rotation policy for log files
#[derive(Debug, Clone)]
pub struct RotationPolicy {
    /// Maximum age of log files in days
    pub max_age_days: u32,
    /// Maximum number of log files to keep
    pub max_files: usize,
    /// Maximum total size of all log files in bytes
    pub max_total_size: u64,
}

impl Default for RotationPolicy {
    fn default() -> Self {
        Self {
            max_age_days: 30,
            max_files: 100,
            max_total_size: 1024 * 1024 * 1024, // 1 GB
        }
    }
}

/// Clean up old log files according to policy
pub async fn cleanup_logs(log_dir: &PathBuf, policy: &RotationPolicy) -> Result<usize> {
    let mut cleaned = 0;

    if !log_dir.exists() {
        return Ok(0);
    }

    let now = Utc::now();
    let cutoff_time = now - Duration::days(policy.max_age_days as i64);

    // Get all log files
    let mut log_files: Vec<(PathBuf, DateTime<Utc>, u64)> = Vec::new();

    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("log") {
            let metadata = std::fs::metadata(&path)?;
            let modified: DateTime<Utc> = metadata.modified()?.into();
            log_files.push((path, modified, metadata.len()));
        }
    }

    // Sort by modification time (newest first)
    log_files.sort_by(|a, b| b.1.cmp(&a.1));

    // Remove files older than max_age_days
    for (path, modified, _) in log_files.iter() {
        if *modified < cutoff_time {
            tracing::info!("Removing old log file: {}", path.display());
            std::fs::remove_file(path)?;
            cleaned += 1;
        }
    }

    // Reload remaining files
    log_files.retain(|(_path, modified, _)| *modified >= cutoff_time);

    // Keep only max_files newest
    if log_files.len() > policy.max_files {
        for (path, _, _) in log_files.iter().skip(policy.max_files) {
            tracing::info!("Removing excess log file: {}", path.display());
            std::fs::remove_file(path)?;
            cleaned += 1;
        }
        log_files.truncate(policy.max_files);
    }

    // Check total size
    let total_size: u64 = log_files.iter().map(|(_, _, size)| size).sum();
    if total_size > policy.max_total_size {
        // Remove oldest files until under limit
        let mut current_size = total_size;
        for (path, _, size) in log_files.iter().rev() {
            if current_size <= policy.max_total_size {
                break;
            }
            tracing::info!("Removing log file to reduce total size: {}", path.display());
            std::fs::remove_file(path)?;
            current_size -= size;
            cleaned += 1;
        }
    }

    if cleaned > 0 {
        tracing::info!("Cleaned up {} log files", cleaned);
    }

    Ok(cleaned)
}
