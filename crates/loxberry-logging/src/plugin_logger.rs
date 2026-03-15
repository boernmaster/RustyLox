//! Plugin-specific logging

use std::path::PathBuf;
use loxberry_core::Result;

/// Create a logger for a specific plugin
pub struct PluginLogger {
    plugin_name: String,
    log_dir: PathBuf,
}

impl PluginLogger {
    pub fn new(plugin_name: String, log_dir: PathBuf) -> Self {
        Self {
            plugin_name,
            log_dir,
        }
    }

    /// Get the log file path for this plugin
    pub fn log_file_path(&self) -> PathBuf {
        self.log_dir.join(format!("{}.log", self.plugin_name))
    }

    /// Write a log entry for the plugin
    pub async fn log(&self, level: LogLevel, message: &str) -> Result<()> {
        use std::io::Write;

        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
        let log_line = format!(
            "[{}] [{}] [{}] {}\n",
            timestamp,
            level.as_str(),
            self.plugin_name,
            message
        );

        // Ensure directory exists
        if let Some(parent) = self.log_file_path().parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Append to file
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log_file_path())?;

        file.write_all(log_line.as_bytes())?;

        Ok(())
    }

    /// Read the last N lines from the plugin log
    pub async fn tail(&self, lines: usize) -> Result<Vec<String>> {
        let path = self.log_file_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let all_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        let start = all_lines.len().saturating_sub(lines);
        Ok(all_lines[start..].to_vec())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}
