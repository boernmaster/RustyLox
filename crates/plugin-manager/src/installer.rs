//! Plugin installer
//!
//! Orchestrates the complete plugin installation process

use crate::config_parser::PluginConfig;
use crate::database::{calculate_plugin_md5, PluginDatabase, PluginDirectories, PluginEntry};
use crate::directory_manager::DirectoryManager;
use crate::lifecycle::{LifecycleHook, LifecycleManager};
use loxberry_core::{Error, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::fs;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

/// Install action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallAction {
    /// Fresh installation
    Install,
    /// Upgrade existing plugin
    Upgrade,
    /// Reinstall (same version)
    Reinstall,
}

/// Installation request
#[derive(Debug)]
pub struct InstallRequest {
    /// Path to ZIP file
    pub zip_path: PathBuf,
    /// Installation action
    pub action: InstallAction,
    /// Force installation even if version check fails
    pub force: bool,
}

/// Plugin installer
pub struct PluginInstaller {
    lbhomedir: PathBuf,
    db_path: PathBuf,
    lifecycle_manager: LifecycleManager,
    directory_manager: DirectoryManager,
}

impl PluginInstaller {
    /// Create a new plugin installer
    pub fn new(lbhomedir: impl Into<PathBuf>) -> Self {
        let lbhomedir = lbhomedir.into();
        let db_path = lbhomedir.join("data/system/plugindatabase.json");

        Self {
            lifecycle_manager: LifecycleManager::new(&lbhomedir),
            directory_manager: DirectoryManager::new(&lbhomedir),
            lbhomedir,
            db_path,
        }
    }

    /// Install a plugin from a ZIP file
    pub async fn install(&self, request: InstallRequest) -> Result<PluginEntry> {
        info!("Starting plugin installation from: {}", request.zip_path.display());

        // Load plugin database
        let mut db = PluginDatabase::load(&self.db_path).await?;

        // Extract ZIP to temp directory
        let temp_dir = self.extract_zip(&request.zip_path).await?;
        let plugin_dir = temp_dir.path();

        // Parse plugin.cfg
        let plugin_cfg_path = plugin_dir.join("plugin.cfg");
        if !plugin_cfg_path.exists() {
            return Err(Error::plugin("plugin.cfg not found in archive"));
        }

        let config = PluginConfig::parse(&plugin_cfg_path)?;
        info!(
            "Parsed plugin: {} v{} (folder: {})",
            config.plugin.name, config.plugin.version, config.plugin.folder
        );

        // Calculate MD5
        let md5 = calculate_plugin_md5(
            &config.author.name,
            &config.author.email,
            &config.plugin.name,
            &config.plugin.folder,
        );
        debug!("Plugin MD5: {}", md5);

        // Check if plugin already exists
        let existing = db.find_by_md5(&md5);
        match (existing, request.action) {
            (Some(existing), InstallAction::Install) => {
                if !request.force {
                    return Err(Error::plugin(format!(
                        "Plugin already installed (version {}). Use upgrade or force install.",
                        existing.version
                    )));
                }
                info!("Force installing over existing version {}", existing.version);
            }
            (None, InstallAction::Upgrade | InstallAction::Reinstall) => {
                return Err(Error::plugin(
                    "Cannot upgrade/reinstall: plugin not found in database",
                ));
            }
            (Some(existing), InstallAction::Upgrade) => {
                info!("Upgrading from version {} to {}", existing.version, config.plugin.version);
            }
            (Some(existing), InstallAction::Reinstall) => {
                info!("Reinstalling version {}", existing.version);
            }
            (None, InstallAction::Install) => {
                info!("Fresh installation");
            }
        }

        // Execute preroot hook
        if let Some(result) = self
            .lifecycle_manager
            .execute_hook(LifecycleHook::PreRoot, plugin_dir, &config.plugin.folder)
            .await?
        {
            if !result.success {
                return Err(Error::plugin(format!(
                    "PreRoot hook failed with exit code {:?}",
                    result.exit_code
                )));
            }
        }

        // Execute preinstall hook
        if let Some(result) = self
            .lifecycle_manager
            .execute_hook(LifecycleHook::PreInstall, plugin_dir, &config.plugin.folder)
            .await?
        {
            if !result.success {
                return Err(Error::plugin(format!(
                    "PreInstall hook failed with exit code {:?}",
                    result.exit_code
                )));
            }
        }

        // Create plugin directory structure
        let paths = self
            .directory_manager
            .create_plugin_structure(&config.plugin.folder)
            .await?;

        // Copy plugin files to their destinations
        self.copy_plugin_files(plugin_dir, &config.plugin.folder)
            .await?;

        // Execute postinstall hook
        let final_plugin_dir = self.lbhomedir.join("bin/plugins").join(&config.plugin.folder);
        if let Some(result) = self
            .lifecycle_manager
            .execute_hook(
                LifecycleHook::PostInstall,
                &final_plugin_dir,
                &config.plugin.folder,
            )
            .await?
        {
            if !result.success {
                warn!(
                    "PostInstall hook failed with exit code {:?}",
                    result.exit_code
                );
                // Don't fail installation on postinstall failure, just warn
            }
        }

        // Execute postroot hook
        if let Some(result) = self
            .lifecycle_manager
            .execute_hook(
                LifecycleHook::PostRoot,
                &final_plugin_dir,
                &config.plugin.folder,
            )
            .await?
        {
            if !result.success {
                warn!(
                    "PostRoot hook failed with exit code {:?}",
                    result.exit_code
                );
                // Don't fail installation on postroot failure, just warn
            }
        }

        // Parse autoupdate setting (0=n/a, 1=disabled, 2=notify, 3=release, 4=prerelease)
        let autoupdate = config
            .plugin
            .autoupdate
            .as_ref()
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(0);

        // Create plugin entry for database
        let plugin_entry = PluginEntry {
            md5: md5.clone(),
            author_name: config.author.name.clone(),
            author_email: config.author.email.clone(),
            version: config.plugin.version.clone(),
            name: config.plugin.name.clone(),
            folder: config.plugin.folder.clone(),
            title: config.plugin.title.clone(),
            interface: config.plugin.interface.clone(),
            autoupdate,
            releasecfg: config.plugin.releasecfg.clone(),
            prereleasecfg: config.plugin.prereleasecfg.clone(),
            loglevel: config.plugin.loglevel.clone().unwrap_or_else(|| "6".to_string()),
            directories: PluginDirectories {
                htmlauth: paths.lbphtmlauthdir.clone(),
                html: paths.lbphtmldir.clone(),
                template: paths.lbptemplatedir.clone(),
                data: paths.lbpdatadir.clone(),
                log: paths.lbplogdir.clone(),
                config: paths.lbpconfigdir.clone(),
                bin: paths.lbpbindir.clone(),
            },
            install_timestamp: match request.action {
                InstallAction::Install => Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                ),
                InstallAction::Upgrade | InstallAction::Reinstall => {
                    existing.and_then(|e| e.install_timestamp)
                }
            },
            update_timestamp: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            ),
        };

        // Update database
        db.upsert(plugin_entry.clone());
        db.save(&self.db_path).await?;

        info!("Successfully installed plugin: {} v{}", config.plugin.name, config.plugin.version);

        Ok(plugin_entry)
    }

    /// Uninstall a plugin
    pub async fn uninstall(&self, md5: &str) -> Result<()> {
        info!("Uninstalling plugin with MD5: {}", md5);

        // Load plugin database
        let mut db = PluginDatabase::load(&self.db_path).await?;

        // Find plugin
        let plugin = db
            .find_by_md5(md5)
            .ok_or_else(|| Error::plugin("Plugin not found in database"))?;

        let folder = plugin.folder.clone();
        let plugin_dir = self.lbhomedir.join("bin/plugins").join(&folder);

        info!("Uninstalling plugin: {} ({})", plugin.name, folder);

        // Execute uninstall hook
        if plugin_dir.exists() {
            if let Some(result) = self
                .lifecycle_manager
                .execute_hook(LifecycleHook::Uninstall, &plugin_dir, &folder)
                .await?
            {
                if !result.success {
                    warn!(
                        "Uninstall hook failed with exit code {:?}",
                        result.exit_code
                    );
                    // Continue with uninstallation even if hook fails
                }
            }
        }

        // Remove plugin directories
        self.directory_manager
            .remove_plugin_structure(&folder)
            .await?;

        // Remove from database
        db.remove(md5);
        db.save(&self.db_path).await?;

        info!("Successfully uninstalled plugin: {}", folder);

        Ok(())
    }

    /// List all installed plugins
    pub async fn list(&self) -> Result<Vec<PluginEntry>> {
        let db = PluginDatabase::load(&self.db_path).await?;
        Ok(db.list().into_iter().cloned().collect())
    }

    /// Get plugin by MD5
    pub async fn get(&self, md5: &str) -> Result<Option<PluginEntry>> {
        let db = PluginDatabase::load(&self.db_path).await?;
        Ok(db.find_by_md5(md5).cloned())
    }

    /// Extract ZIP file to temporary directory
    async fn extract_zip(&self, zip_path: &Path) -> Result<TempDir> {
        info!("Extracting ZIP file: {}", zip_path.display());

        let temp_dir = TempDir::new()
            .map_err(|e| Error::plugin(format!("Failed to create temp directory: {}", e)))?;

        let file = std::fs::File::open(zip_path)
            .map_err(|e| Error::plugin(format!("Failed to open ZIP file: {}", e)))?;

        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| Error::plugin(format!("Failed to read ZIP archive: {}", e)))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| Error::plugin(format!("Failed to read ZIP entry: {}", e)))?;

            let outpath = match file.enclosed_name() {
                Some(path) => temp_dir.path().join(path),
                None => continue, // Skip invalid paths
            };

            if file.name().ends_with('/') {
                // Directory
                std::fs::create_dir_all(&outpath)
                    .map_err(|e| Error::plugin(format!("Failed to create directory: {}", e)))?;
            } else {
                // File
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        Error::plugin(format!("Failed to create parent directory: {}", e))
                    })?;
                }

                let mut outfile = std::fs::File::create(&outpath)
                    .map_err(|e| Error::plugin(format!("Failed to create file: {}", e)))?;

                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| Error::plugin(format!("Failed to extract file: {}", e)))?;
            }

            // Set Unix permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))
                        .ok(); // Ignore errors
                }
            }
        }

        info!("Extracted {} files to temp directory", archive.len());
        Ok(temp_dir)
    }

    /// Copy plugin files to their destinations
    async fn copy_plugin_files(&self, source_dir: &Path, folder: &str) -> Result<()> {
        info!("Copying plugin files for: {}", folder);

        // Map of source directories to destination directories
        let mappings = vec![
            ("webfrontend/htmlauth", format!("webfrontend/htmlauth/plugins/{}", folder)),
            ("webfrontend/html", format!("webfrontend/html/plugins/{}", folder)),
            ("templates", format!("templates/plugins/{}", folder)),
            ("data", format!("data/plugins/{}", folder)),
            ("config", format!("config/plugins/{}", folder)),
            ("bin", format!("bin/plugins/{}", folder)),
        ];

        for (src_rel, dst_rel) in mappings {
            let src = source_dir.join(src_rel);
            if !src.exists() {
                debug!("Skipping non-existent directory: {}", src_rel);
                continue;
            }

            let dst = self.lbhomedir.join(&dst_rel);
            fs::create_dir_all(&dst)
                .await
                .map_err(|e| Error::plugin(format!("Failed to create directory {}: {}", dst_rel, e)))?;

            self.copy_dir_recursive(&src, &dst).await?;
            info!("Copied {} -> {}", src_rel, dst_rel);
        }

        Ok(())
    }

    /// Recursively copy directory contents
    async fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> Result<()> {
        for entry in WalkDir::new(src).min_depth(1) {
            let entry = entry
                .map_err(|e| Error::plugin(format!("Failed to walk directory: {}", e)))?;

            let path = entry.path();
            let relative = path
                .strip_prefix(src)
                .map_err(|e| Error::plugin(format!("Failed to compute relative path: {}", e)))?;

            let target = dst.join(relative);

            if path.is_dir() {
                fs::create_dir_all(&target)
                    .await
                    .map_err(|e| Error::plugin(format!("Failed to create directory: {}", e)))?;
            } else {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)
                        .await
                        .map_err(|e| Error::plugin(format!("Failed to create parent: {}", e)))?;
                }

                fs::copy(path, &target)
                    .await
                    .map_err(|e| Error::plugin(format!("Failed to copy file: {}", e)))?;

                debug!("Copied: {} -> {}", path.display(), target.display());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_installer_workflow() {
        // This is an integration test that would require a real plugin ZIP
        // For now, just verify the installer can be created
        let temp_dir = TempDir::new().unwrap();
        let installer = PluginInstaller::new(temp_dir.path());

        // Verify list works on empty database
        let plugins = installer.list().await.unwrap();
        assert_eq!(plugins.len(), 0);
    }
}
