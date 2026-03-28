//! Plugin lifecycle hook execution
//!
//! Executes lifecycle hooks during plugin installation and removal

use rustylox_core::{Error, PluginPaths, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Lifecycle hook types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleHook {
    /// Executed before installation (as root)
    PreRoot,
    /// Executed before installation (as loxberry user)
    PreInstall,
    /// Executed before upgrade (as loxberry user)
    PreUpgrade,
    /// Executed after installation (as loxberry user)
    PostInstall,
    /// Executed after upgrade (as loxberry user)
    PostUpgrade,
    /// Executed after installation (as root)
    PostRoot,
    /// Executed during uninstallation
    Uninstall,
}

impl LifecycleHook {
    /// Get the script filename for this hook
    pub fn script_name(&self) -> &str {
        match self {
            LifecycleHook::PreRoot => "preroot.sh",
            LifecycleHook::PreInstall => "preinstall.sh",
            LifecycleHook::PreUpgrade => "preupgrade.sh",
            LifecycleHook::PostInstall => "postinstall.sh",
            LifecycleHook::PostUpgrade => "postupgrade.sh",
            LifecycleHook::PostRoot => "postroot.sh",
            LifecycleHook::Uninstall => "uninstall.sh",
        }
    }

    /// Check if this hook requires root privileges
    pub fn requires_root(&self) -> bool {
        matches!(self, LifecycleHook::PreRoot | LifecycleHook::PostRoot)
    }
}

/// Lifecycle hook manager
pub struct LifecycleManager {
    lbhomedir: PathBuf,
}

/// Result from hook execution
#[derive(Debug)]
pub struct HookResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new(lbhomedir: impl Into<PathBuf>) -> Self {
        Self {
            lbhomedir: lbhomedir.into(),
        }
    }

    /// Execute a lifecycle hook if it exists
    ///
    /// Arguments passed to hooks match original LoxBerry plugininstall.pl:
    /// ARGV[0] = tempfile (session identifier)
    /// ARGV[1] = plugin name
    /// ARGV[2] = plugin folder
    /// ARGV[3] = plugin version
    /// ARGV[4] = lbhomedir
    /// ARGV[5] = source directory (temp extraction folder)
    pub async fn execute_hook(
        &self,
        hook: LifecycleHook,
        plugin_dir: &Path,
        plugin_folder: &str,
    ) -> Result<Option<HookResult>> {
        self.execute_hook_with_args(hook, plugin_dir, plugin_folder, "", "", None)
            .await
    }

    /// Execute a lifecycle hook with full LoxBerry-compatible arguments
    pub async fn execute_hook_with_args(
        &self,
        hook: LifecycleHook,
        plugin_dir: &Path,
        plugin_folder: &str,
        plugin_name: &str,
        plugin_version: &str,
        source_dir: Option<&Path>,
    ) -> Result<Option<HookResult>> {
        let script_path = plugin_dir.join(hook.script_name());

        if !script_path.exists() {
            debug!("Hook script not found: {}", script_path.display());
            return Ok(None);
        }

        info!(
            "Executing {} hook: {}",
            hook.script_name(),
            script_path.display()
        );

        // Make script executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&script_path)
                .await
                .map_err(|e| Error::plugin(format!("Failed to get script metadata: {}", e)))?
                .permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&script_path, perms)
                .await
                .map_err(|e| Error::plugin(format!("Failed to set script permissions: {}", e)))?;
        }

        // Build environment variables
        let env_vars = self.build_plugin_env(plugin_folder);

        // Generate a session identifier (matches original LoxBerry tempfile)
        let tempfile_id: String = (0..10)
            .map(|_| {
                let idx = rand_char();
                (b'a' + idx) as char
            })
            .collect();

        // Execute the script with positional arguments matching original LoxBerry
        let mut cmd = Command::new("bash");
        cmd.arg(&script_path)
            .arg(&tempfile_id) // ARGV[0]: tempfile session ID
            .arg(plugin_name) // ARGV[1]: plugin name
            .arg(plugin_folder) // ARGV[2]: plugin folder
            .arg(plugin_version) // ARGV[3]: plugin version
            .arg(self.lbhomedir.display().to_string()) // ARGV[4]: lbhomedir
            .arg(
                source_dir
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            ) // ARGV[5]: source dir
            .envs(&env_vars)
            .current_dir(plugin_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Note: For root hooks (PreRoot, PostRoot), we would need to use sudo
        // This is simplified for the Docker environment where we run as loxberry user
        if hook.requires_root() {
            warn!(
                "Root hook requested but running as loxberry user: {}",
                hook.script_name()
            );
        }

        let output = cmd.output().await.map_err(|e| {
            Error::plugin(format!(
                "Failed to execute hook {}: {}",
                hook.script_name(),
                e
            ))
        })?;

        let result = HookResult {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        };

        if result.success {
            info!("Hook {} completed successfully", hook.script_name());
            debug!("Hook output: {}", result.stdout);
        } else {
            warn!(
                "Hook {} failed with exit code {:?}",
                hook.script_name(),
                result.exit_code
            );
            warn!("Hook stderr: {}", result.stderr);
        }

        Ok(Some(result))
    }

    /// Build environment variables for plugin execution
    fn build_plugin_env(&self, plugin_folder: &str) -> HashMap<String, String> {
        let paths = PluginPaths::new(&self.lbhomedir.display().to_string(), plugin_folder);
        paths.to_env_vars(&self.lbhomedir.display().to_string())
    }

    /// Execute preinstall hook
    pub async fn execute_preinstall(
        &self,
        plugin_dir: &Path,
        plugin_folder: &str,
    ) -> Result<Option<HookResult>> {
        self.execute_hook(LifecycleHook::PreInstall, plugin_dir, plugin_folder)
            .await
    }

    /// Execute postinstall hook
    pub async fn execute_postinstall(
        &self,
        plugin_dir: &Path,
        plugin_folder: &str,
    ) -> Result<Option<HookResult>> {
        self.execute_hook(LifecycleHook::PostInstall, plugin_dir, plugin_folder)
            .await
    }

    /// Execute uninstall hook
    pub async fn execute_uninstall(
        &self,
        plugin_dir: &Path,
        plugin_folder: &str,
    ) -> Result<Option<HookResult>> {
        self.execute_hook(LifecycleHook::Uninstall, plugin_dir, plugin_folder)
            .await
    }
}

/// Simple random character generator for session IDs (a-z)
fn rand_char() -> u8 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos % 26) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_execute_hook() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("testplugin");
        fs::create_dir_all(&plugin_dir).await.unwrap();

        // Create a simple test hook
        let hook_script = plugin_dir.join("postinstall.sh");
        fs::write(&hook_script, "#!/bin/bash\necho 'Hello from hook'\nexit 0")
            .await
            .unwrap();

        let manager = LifecycleManager::new(temp_dir.path());

        let result = manager
            .execute_hook(LifecycleHook::PostInstall, &plugin_dir, "testplugin")
            .await
            .unwrap();

        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.success);
        assert!(result.stdout.contains("Hello from hook"));
    }

    #[tokio::test]
    async fn test_missing_hook() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("testplugin");
        fs::create_dir_all(&plugin_dir).await.unwrap();

        let manager = LifecycleManager::new(temp_dir.path());

        let result = manager
            .execute_hook(LifecycleHook::PostInstall, &plugin_dir, "testplugin")
            .await
            .unwrap();

        // Hook doesn't exist, should return None
        assert!(result.is_none());
    }
}
