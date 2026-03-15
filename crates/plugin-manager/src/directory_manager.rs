//! Directory manager for plugin isolation
//!
//! Creates and manages isolated directory structures for plugins

use loxberry_core::{Error, PluginPaths, Result};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info};

/// Manages plugin directory creation and deletion
pub struct DirectoryManager {
    lbhomedir: PathBuf,
}

impl DirectoryManager {
    /// Create a new directory manager
    pub fn new(lbhomedir: impl Into<PathBuf>) -> Self {
        Self {
            lbhomedir: lbhomedir.into(),
        }
    }

    /// Create complete directory structure for a plugin
    pub async fn create_plugin_structure(&self, folder: &str) -> Result<PluginPaths> {
        info!("Creating directory structure for plugin: {}", folder);

        let paths = PluginPaths::new(&self.lbhomedir.display().to_string(), folder);

        // Create all plugin directories
        let dirs = vec![
            &paths.lbphtmlauthdir,
            &paths.lbphtmldir,
            &paths.lbptemplatedir,
            &paths.lbpdatadir,
            &paths.lbplogdir,
            &paths.lbpconfigdir,
            &paths.lbpbindir,
        ];

        for dir in dirs {
            debug!("Creating directory: {}", dir);
            fs::create_dir_all(dir)
                .await
                .map_err(|e| Error::plugin(format!("Failed to create directory {}: {}", dir, e)))?;
        }

        info!("Successfully created directory structure for: {}", folder);
        Ok(paths)
    }

    /// Remove complete directory structure for a plugin
    pub async fn remove_plugin_structure(&self, folder: &str) -> Result<()> {
        info!("Removing directory structure for plugin: {}", folder);

        let paths = PluginPaths::new(&self.lbhomedir.display().to_string(), folder);

        // Remove all plugin directories
        let dirs = vec![
            &paths.lbphtmlauthdir,
            &paths.lbphtmldir,
            &paths.lbptemplatedir,
            &paths.lbpdatadir,
            &paths.lbplogdir,
            &paths.lbpconfigdir,
            &paths.lbpbindir,
        ];

        for dir in dirs {
            if Path::new(dir).exists() {
                debug!("Removing directory: {}", dir);
                fs::remove_dir_all(dir).await.map_err(|e| {
                    Error::plugin(format!("Failed to remove directory {}: {}", dir, e))
                })?;
            }
        }

        info!("Successfully removed directory structure for: {}", folder);
        Ok(())
    }

    /// Check if plugin directories already exist
    pub async fn plugin_exists(&self, folder: &str) -> bool {
        let paths = PluginPaths::new(&self.lbhomedir.display().to_string(), folder);
        Path::new(&paths.lbpdatadir).exists()
    }

    /// Set permissions on plugin directories
    ///
    /// In a real deployment, this would use `chown` to set loxberry:loxberry ownership
    /// For now, we'll just ensure the directories are writable
    pub async fn set_permissions(&self, folder: &str) -> Result<()> {
        debug!("Setting permissions for plugin: {}", folder);

        // In production Docker environment, files are already owned by loxberry user
        // Additional permission setting would go here if needed

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_and_remove_plugin_structure() {
        let temp_dir = TempDir::new().unwrap();
        let manager = DirectoryManager::new(temp_dir.path());

        // Create plugin structure
        let paths = manager.create_plugin_structure("testplugin").await.unwrap();

        // Verify directories were created
        assert!(Path::new(&paths.lbpdatadir).exists());
        assert!(Path::new(&paths.lbpconfigdir).exists());
        assert!(Path::new(&paths.lbplogdir).exists());

        // Remove plugin structure
        manager.remove_plugin_structure("testplugin").await.unwrap();

        // Verify directories were removed
        assert!(!Path::new(&paths.lbpdatadir).exists());
        assert!(!Path::new(&paths.lbpconfigdir).exists());
    }

    #[tokio::test]
    async fn test_plugin_exists() {
        let temp_dir = TempDir::new().unwrap();
        let manager = DirectoryManager::new(temp_dir.path());

        // Plugin doesn't exist initially
        assert!(!manager.plugin_exists("testplugin").await);

        // Create plugin
        manager.create_plugin_structure("testplugin").await.unwrap();

        // Now it exists
        assert!(manager.plugin_exists("testplugin").await);
    }
}
