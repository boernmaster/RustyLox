//! Plugin daemon management
//!
//! Manages the lifecycle of plugin background processes (daemons).
//! Plugins can have a daemon/daemon.pl (or .sh, .php) that runs continuously.

use crate::database::PluginEntry;
use crate::environment::build_plugin_env;
use loxberry_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Daemon status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DaemonStatus {
    Running,
    Stopped,
    Unknown,
}

/// Information about a running daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonInfo {
    pub plugin_name: String,
    pub plugin_folder: String,
    pub status: DaemonStatus,
    pub pid: Option<u32>,
    pub uptime_seconds: Option<u64>,
}

/// Manages plugin daemon processes
pub struct DaemonManager {
    lbhomedir: PathBuf,
    /// Map of plugin folder -> PID (in-memory tracking)
    running_pids: Arc<RwLock<HashMap<String, u32>>>,
}

impl DaemonManager {
    /// Create a new daemon manager
    pub fn new(lbhomedir: impl Into<PathBuf>) -> Self {
        Self {
            lbhomedir: lbhomedir.into(),
            running_pids: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start a plugin daemon
    pub async fn start(&self, plugin: &PluginEntry) -> Result<DaemonInfo> {
        info!("Starting daemon for plugin: {}", plugin.folder);

        let daemon_script = self.find_daemon_script(&plugin.folder)?;
        debug!("Found daemon script: {}", daemon_script.display());

        let env = build_plugin_env(plugin, &self.lbhomedir);
        let interpreter = detect_interpreter(&daemon_script)?;

        let child = tokio::process::Command::new(&interpreter)
            .arg(&daemon_script)
            .envs(&env)
            .current_dir(&self.lbhomedir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                Error::plugin(format!(
                    "Failed to start daemon for {}: {}",
                    plugin.folder, e
                ))
            })?;

        let pid = child.id().unwrap_or(0);
        info!(
            "Started daemon for plugin {} with PID {}",
            plugin.folder, pid
        );

        // Store PID
        {
            let mut pids = self.running_pids.write().await;
            pids.insert(plugin.folder.clone(), pid);
        }

        // Write PID file
        self.write_pid_file(&plugin.folder, pid).await.ok();

        // Detach the process (don't wait for it)
        drop(child);

        Ok(DaemonInfo {
            plugin_name: plugin.name.clone(),
            plugin_folder: plugin.folder.clone(),
            status: DaemonStatus::Running,
            pid: Some(pid),
            uptime_seconds: Some(0),
        })
    }

    /// Stop a plugin daemon
    pub async fn stop(&self, plugin: &PluginEntry) -> Result<DaemonInfo> {
        info!("Stopping daemon for plugin: {}", plugin.folder);

        let pid = self.get_pid(&plugin.folder).await;

        if let Some(pid) = pid {
            // Send SIGTERM via the kill command (portable Unix approach)
            let _ = tokio::process::Command::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .output()
                .await;

            // Remove from tracking
            {
                let mut pids = self.running_pids.write().await;
                pids.remove(&plugin.folder);
            }

            // Remove PID file
            self.remove_pid_file(&plugin.folder).await.ok();

            info!("Stopped daemon for plugin {} (PID {})", plugin.folder, pid);
        } else {
            warn!("No running daemon found for plugin: {}", plugin.folder);
        }

        Ok(DaemonInfo {
            plugin_name: plugin.name.clone(),
            plugin_folder: plugin.folder.clone(),
            status: DaemonStatus::Stopped,
            pid: None,
            uptime_seconds: None,
        })
    }

    /// Restart a plugin daemon
    pub async fn restart(&self, plugin: &PluginEntry) -> Result<DaemonInfo> {
        info!("Restarting daemon for plugin: {}", plugin.folder);
        self.stop(plugin).await.ok();
        // Brief pause to allow process cleanup
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        self.start(plugin).await
    }

    /// Get daemon status for a plugin
    pub async fn status(&self, plugin: &PluginEntry) -> DaemonInfo {
        let pid = self.get_pid(&plugin.folder).await;

        let (status, actual_pid) = if let Some(pid) = pid {
            if is_process_running(pid) {
                (DaemonStatus::Running, Some(pid))
            } else {
                // Process died, clean up
                {
                    let mut pids = self.running_pids.write().await;
                    pids.remove(&plugin.folder);
                }
                self.remove_pid_file(&plugin.folder).await.ok();
                (DaemonStatus::Stopped, None)
            }
        } else {
            (DaemonStatus::Stopped, None)
        };

        DaemonInfo {
            plugin_name: plugin.name.clone(),
            plugin_folder: plugin.folder.clone(),
            status,
            pid: actual_pid,
            uptime_seconds: None,
        }
    }

    /// Get daemon logs (last N lines)
    pub async fn get_logs(&self, plugin: &PluginEntry, lines: usize) -> Result<String> {
        let log_file = self
            .lbhomedir
            .join("log/plugins")
            .join(&plugin.folder)
            .join("daemon.log");

        if !log_file.exists() {
            return Ok(String::new());
        }

        let content = tokio::fs::read_to_string(&log_file)
            .await
            .map_err(|e| Error::plugin(format!("Failed to read daemon log: {}", e)))?;

        let log_lines: Vec<&str> = content.lines().collect();
        let start = if log_lines.len() > lines {
            log_lines.len() - lines
        } else {
            0
        };

        Ok(log_lines[start..].join("\n"))
    }

    /// Check if plugin has a daemon script
    pub fn has_daemon(&self, folder: &str) -> bool {
        self.find_daemon_script(folder).is_ok()
    }

    /// Get PID from memory or PID file
    async fn get_pid(&self, folder: &str) -> Option<u32> {
        // Check in-memory first
        {
            let pids = self.running_pids.read().await;
            if let Some(&pid) = pids.get(folder) {
                return Some(pid);
            }
        }

        // Fall back to PID file
        let pid_file = self.pid_file_path(folder);
        if pid_file.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&pid_file).await {
                if let Ok(pid) = content.trim().parse::<u32>() {
                    // Verify the process is still running
                    if is_process_running(pid) {
                        let mut pids = self.running_pids.write().await;
                        pids.insert(folder.to_string(), pid);
                        return Some(pid);
                    }
                }
            }
        }

        None
    }

    /// Write PID file
    async fn write_pid_file(&self, folder: &str, pid: u32) -> Result<()> {
        let pid_file = self.pid_file_path(folder);
        if let Some(parent) = pid_file.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        tokio::fs::write(&pid_file, pid.to_string())
            .await
            .map_err(|e| Error::plugin(format!("Failed to write PID file: {}", e)))?;
        Ok(())
    }

    /// Remove PID file
    async fn remove_pid_file(&self, folder: &str) -> Result<()> {
        let pid_file = self.pid_file_path(folder);
        if pid_file.exists() {
            tokio::fs::remove_file(&pid_file)
                .await
                .map_err(|e| Error::plugin(format!("Failed to remove PID file: {}", e)))?;
        }
        Ok(())
    }

    /// Get PID file path for a plugin
    fn pid_file_path(&self, folder: &str) -> PathBuf {
        self.lbhomedir
            .join("log/plugins")
            .join(folder)
            .join("daemon.pid")
    }

    /// Find daemon script for a plugin (checks multiple extensions)
    fn find_daemon_script(&self, folder: &str) -> Result<PathBuf> {
        let daemon_dir = self
            .lbhomedir
            .join("bin/plugins")
            .join(folder)
            .join("daemon");

        let candidates = [
            daemon_dir.join("daemon.pl"),
            daemon_dir.join("daemon.sh"),
            daemon_dir.join("daemon.php"),
            daemon_dir.join("daemon.py"),
        ];

        for candidate in &candidates {
            if candidate.exists() {
                return Ok(candidate.clone());
            }
        }

        Err(Error::plugin(format!(
            "No daemon script found for plugin {}",
            folder
        )))
    }
}

/// Detect the interpreter for a script based on its extension or shebang
fn detect_interpreter(script: &Path) -> Result<String> {
    match script.extension().and_then(|e| e.to_str()) {
        Some("pl") => Ok("perl".to_string()),
        Some("php") => Ok("php".to_string()),
        Some("sh") => Ok("bash".to_string()),
        Some("py") => Ok("python3".to_string()),
        _ => Ok("bash".to_string()),
    }
}

/// Check if a process is running by PID
fn is_process_running(pid: u32) -> bool {
    // On Linux, check /proc/<pid> - fast and reliable
    #[cfg(target_os = "linux")]
    {
        Path::new(&format!("/proc/{}", pid)).exists()
    }

    // On other Unix, use kill -0 (signal 0 checks if process exists without sending a signal)
    #[cfg(all(unix, not(target_os = "linux")))]
    {
        // kill -0 <pid> returns 0 if process exists, non-zero otherwise
        std::process::Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        // Windows: use tasklist to check
        let _ = pid;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_interpreter() {
        assert_eq!(detect_interpreter(Path::new("daemon.pl")).unwrap(), "perl");
        assert_eq!(detect_interpreter(Path::new("daemon.sh")).unwrap(), "bash");
        assert_eq!(detect_interpreter(Path::new("daemon.php")).unwrap(), "php");
        assert_eq!(detect_interpreter(Path::new("daemon.py")).unwrap(), "python3");
    }

    #[test]
    fn test_is_process_running_current() {
        // Current process should be running
        let current_pid = std::process::id();
        assert!(is_process_running(current_pid));
    }

    #[test]
    fn test_is_process_not_running() {
        // PID 9999999 almost certainly doesn't exist
        assert!(!is_process_running(9999999));
    }
}
