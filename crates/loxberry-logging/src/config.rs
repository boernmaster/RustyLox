//! Logging configuration

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,

    /// Maximum age of log files in days
    pub max_age_days: u32,

    /// Maximum number of log files to keep
    pub max_files: usize,

    /// Maximum total size in MB
    pub max_size_mb: u64,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            max_age_days: 30,
            max_files: 100,
            max_size_mb: 1024,
        }
    }
}
