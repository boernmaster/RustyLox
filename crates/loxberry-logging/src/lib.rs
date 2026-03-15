//! Logging system for LoxBerry
//!
//! This crate provides:
//! - Structured logging with rotation
//! - Plugin-specific log files
//! - Log level management
//! - Web UI log access

pub mod rotation;
pub mod plugin_logger;
pub mod config;

use loxberry_core::Result;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize the global logging system
pub fn init_logging(log_dir: PathBuf, log_level: &str) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    // Console output layer
    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_file(false)
        .with_line_number(false);

    // File output layer with rotation
    let file_appender = tracing_appender::rolling::daily(
        log_dir.join("system"),
        "loxberry.log",
    );
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true);

    // Combine layers
    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    tracing::info!("Logging system initialized");
    Ok(())
}

/// Get log files from a directory
pub fn get_log_files(log_dir: &PathBuf) -> Result<Vec<LogFile>> {
    let mut files = Vec::new();

    if !log_dir.exists() {
        return Ok(files);
    }

    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("log") {
            let metadata = std::fs::metadata(&path)?;
            files.push(LogFile {
                name: path.file_name().unwrap().to_string_lossy().to_string(),
                path: path.clone(),
                size: metadata.len(),
                modified: metadata.modified()?.into(),
            });
        }
    }

    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(files)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LogFile {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: chrono::DateTime<chrono::Utc>,
}
