//! Script executor for plugin lifecycle hooks

use loxberry_core::Result;
use std::path::{Path, PathBuf};
use std::process::Output;
use tokio::process::Command;
use crate::database::PluginEntry;
use crate::environment::build_plugin_env;

pub struct PluginExecutor {
    lbhomedir: PathBuf,
}

impl PluginExecutor {
    pub fn new(lbhomedir: PathBuf) -> Self {
        Self { lbhomedir }
    }

    /// Execute a Perl script
    pub async fn execute_perl(
        &self,
        script: &Path,
        plugin: &PluginEntry,
    ) -> Result<Output> {
        let env = build_plugin_env(plugin, &self.lbhomedir);

        tracing::info!("Executing Perl script: {}", script.display());

        let output = Command::new("perl")
            .arg(script)
            .envs(&env)
            .current_dir(&self.lbhomedir)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!("Perl script failed: {}", stderr);
            return Err(loxberry_core::Error::plugin(format!(
                "Script execution failed: {}",
                stderr
            ))
            .into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        tracing::debug!("Perl script output: {}", stdout);

        Ok(output)
    }

    /// Execute a PHP script
    pub async fn execute_php(
        &self,
        script: &Path,
        plugin: &PluginEntry,
    ) -> Result<Output> {
        let env = build_plugin_env(plugin, &self.lbhomedir);

        tracing::info!("Executing PHP script: {}", script.display());

        let output = Command::new("php")
            .arg(script)
            .envs(&env)
            .current_dir(&self.lbhomedir)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!("PHP script failed: {}", stderr);
            return Err(loxberry_core::Error::plugin(format!(
                "Script execution failed: {}",
                stderr
            ))
            .into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        tracing::debug!("PHP script output: {}", stdout);

        Ok(output)
    }

    /// Execute a Bash script
    pub async fn execute_bash(
        &self,
        script: &Path,
        plugin: &PluginEntry,
    ) -> Result<Output> {
        let env = build_plugin_env(plugin, &self.lbhomedir);

        tracing::info!("Executing Bash script: {}", script.display());

        let output = Command::new("bash")
            .arg(script)
            .envs(&env)
            .current_dir(&self.lbhomedir)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!("Bash script failed: {}", stderr);
            return Err(loxberry_core::Error::plugin(format!(
                "Script execution failed: {}",
                stderr
            ))
            .into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        tracing::debug!("Bash script output: {}", stdout);

        Ok(output)
    }

    /// Execute a script based on its extension
    pub async fn execute_script(
        &self,
        script: &Path,
        plugin: &PluginEntry,
    ) -> Result<Output> {
        match script.extension().and_then(|e| e.to_str()) {
            Some("pl") => self.execute_perl(script, plugin).await,
            Some("php") => self.execute_php(script, plugin).await,
            Some("sh") => self.execute_bash(script, plugin).await,
            Some(ext) => {
                tracing::warn!("Unknown script extension: {}, trying bash", ext);
                self.execute_bash(script, plugin).await
            }
            None => {
                // No extension, check shebang
                let shebang = tokio::fs::read_to_string(script).await?;
                let first_line = shebang.lines().next().unwrap_or("");

                if first_line.contains("perl") {
                    self.execute_perl(script, plugin).await
                } else if first_line.contains("php") {
                    self.execute_php(script, plugin).await
                } else {
                    self.execute_bash(script, plugin).await
                }
            }
        }
    }
}
