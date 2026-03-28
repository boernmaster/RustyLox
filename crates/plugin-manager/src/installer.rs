//! Plugin installer
//!
//! Orchestrates the complete plugin installation process

use crate::config_parser::PluginConfig;
use crate::database::{
    calculate_plugin_md5, PluginDatabase, PluginDirectories, PluginEntry, PluginFiles,
};
use crate::directory_manager::DirectoryManager;
use crate::lifecycle::{LifecycleHook, LifecycleManager};
use rustylox_core::{Error, Result};
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
        info!(
            "Starting plugin installation from: {}",
            request.zip_path.display()
        );

        // Load plugin database
        let mut db = PluginDatabase::load(&self.db_path).await?;

        // Extract ZIP to temp directory
        let temp_dir = self.extract_zip(&request.zip_path).await?;
        let extracted_path = temp_dir.path();

        // Find plugin.cfg (may be in root or subdirectory)
        let plugin_cfg_path = self.find_plugin_cfg(extracted_path)?;
        let plugin_dir = plugin_cfg_path
            .parent()
            .ok_or_else(|| Error::plugin("Invalid plugin.cfg path"))?;

        let mut config = PluginConfig::parse(&plugin_cfg_path)?;
        info!(
            "Parsed plugin: {} v{} (folder: {})",
            config.plugin.name, config.plugin.version, config.plugin.folder
        );

        // Sanitize name and folder to [A-Za-z0-9_-] (matches original LoxBerry)
        config.plugin.name = sanitize_identifier(&config.plugin.name);
        config.plugin.folder = sanitize_identifier(&config.plugin.folder);

        // Truncate title to 25 chars (matches original LoxBerry)
        for title in config.plugin.title.values_mut() {
            if title.len() > 25 {
                title.truncate(22);
                title.push_str("...");
            }
        }

        // Validate interface version (original LoxBerry requires 2.0, rejects 1.0)
        let iface_version = config
            .system
            .as_ref()
            .and_then(|s| s.interface.clone())
            .or_else(|| config.plugin.interface.clone());
        if let Some(ref iface) = iface_version {
            if iface == "1.0" {
                return Err(Error::plugin(
                    "Plugin interface version 1.0 is no longer supported. Requires 2.0+",
                ));
            }
        }

        // Validate architecture compatibility
        if let Some(ref sys) = config.system {
            if let Some(ref arch_list) = sys.architecture {
                if !arch_list.is_empty() {
                    let supported = self.check_architecture(arch_list);
                    if !supported {
                        return Err(Error::plugin(format!(
                            "Plugin requires architecture '{}' which is not supported by this system",
                            arch_list
                        )));
                    }
                }
            }

            // Validate LoxBerry version constraints
            if let Some(ref lb_min) = sys.lb_minimum {
                if !lb_min.is_empty() && lb_min != "false" {
                    let system_version = self.get_system_version().await;
                    if let Some(ref sv) = system_version {
                        if version_compare(sv, lb_min) < 0 {
                            warn!("System version {} is below minimum required {}", sv, lb_min);
                            if !request.force {
                                return Err(Error::plugin(format!(
                                    "Plugin requires LoxBerry >= {} but system is {}",
                                    lb_min, sv
                                )));
                            }
                        }
                    }
                }
            }
            if let Some(ref lb_max) = sys.lb_maximum {
                if !lb_max.is_empty() && lb_max != "false" {
                    let system_version = self.get_system_version().await;
                    if let Some(ref sv) = system_version {
                        if version_compare(sv, lb_max) > 0 {
                            warn!(
                                "System version {} is above maximum supported {}",
                                sv, lb_max
                            );
                            if !request.force {
                                return Err(Error::plugin(format!(
                                    "Plugin requires LoxBerry <= {} but system is {}",
                                    lb_max, sv
                                )));
                            }
                        }
                    }
                }
            }
        }

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
                info!(
                    "Force installing over existing version {}",
                    existing.version
                );
            }
            (None, InstallAction::Upgrade | InstallAction::Reinstall) => {
                return Err(Error::plugin(
                    "Cannot upgrade/reinstall: plugin not found in database",
                ));
            }
            (Some(existing), InstallAction::Upgrade) => {
                info!(
                    "Upgrading from version {} to {}",
                    existing.version, config.plugin.version
                );
            }
            (Some(existing), InstallAction::Reinstall) => {
                info!("Reinstalling version {}", existing.version);
            }
            (None, InstallAction::Install) => {
                info!("Fresh installation");
            }
        }

        // Name/folder conflict resolution (matches original LoxBerry plugininstall.pl)
        // If a different plugin already uses the same name or folder, append 3-char MD5 suffix
        let mut orig_name = None;
        let mut orig_folder = None;
        if existing.is_none() {
            let name_conflict = db
                .list()
                .iter()
                .any(|p| p.name == config.plugin.name && p.md5 != md5);
            let folder_conflict = db
                .list()
                .iter()
                .any(|p| p.folder == config.plugin.folder && p.md5 != md5);

            if name_conflict || folder_conflict {
                let suffix = &md5[..3];
                let new_name = format!("{}_{}", config.plugin.name, suffix);
                let new_folder = format!("{}_{}", config.plugin.folder, suffix);

                // Check that the renamed version doesn't also conflict
                let still_conflicts = db
                    .list()
                    .iter()
                    .any(|p| (p.name == new_name || p.folder == new_folder) && p.md5 != md5);

                if still_conflicts {
                    return Err(Error::plugin(format!(
                        "Cannot resolve name/folder conflict for plugin '{}' (folder '{}')",
                        config.plugin.name, config.plugin.folder
                    )));
                }

                warn!(
                    "Name/folder conflict detected. Renaming: {} -> {}, {} -> {}",
                    config.plugin.name, new_name, config.plugin.folder, new_folder
                );
                orig_name = Some(config.plugin.name.clone());
                orig_folder = Some(config.plugin.folder.clone());
                config.plugin.name = new_name;
                config.plugin.folder = new_folder;
            }
        }

        // Execute preroot hook
        if let Some(result) = self
            .lifecycle_manager
            .execute_hook_with_args(
                LifecycleHook::PreRoot,
                plugin_dir,
                &config.plugin.folder,
                &config.plugin.name,
                &config.plugin.version,
                Some(plugin_dir),
            )
            .await?
        {
            if !result.success {
                return Err(Error::plugin(format!(
                    "PreRoot hook failed with exit code {:?}",
                    result.exit_code
                )));
            }
        }

        // Execute preupgrade hook (only for upgrades, matches original LoxBerry)
        if request.action == InstallAction::Upgrade {
            if let Some(result) = self
                .lifecycle_manager
                .execute_hook_with_args(
                    LifecycleHook::PreUpgrade,
                    plugin_dir,
                    &config.plugin.folder,
                    &config.plugin.name,
                    &config.plugin.version,
                    Some(plugin_dir),
                )
                .await?
            {
                if !result.success {
                    warn!(
                        "PreUpgrade hook failed with exit code {:?}",
                        result.exit_code
                    );
                }
            }
        }

        // Execute preinstall hook
        if let Some(result) = self
            .lifecycle_manager
            .execute_hook_with_args(
                LifecycleHook::PreInstall,
                plugin_dir,
                &config.plugin.folder,
                &config.plugin.name,
                &config.plugin.version,
                Some(plugin_dir),
            )
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
        self.copy_plugin_files(plugin_dir, &config).await?;

        // Install APT packages (non-fatal, matches original LoxBerry)
        self.install_apt_packages(plugin_dir).await;

        // Execute postinstall hook (from final installed location)
        let final_plugin_dir = self
            .lbhomedir
            .join("bin/plugins")
            .join(&config.plugin.folder);
        if let Some(result) = self
            .lifecycle_manager
            .execute_hook_with_args(
                LifecycleHook::PostInstall,
                &final_plugin_dir,
                &config.plugin.folder,
                &config.plugin.name,
                &config.plugin.version,
                Some(plugin_dir),
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

        // Execute postupgrade hook (only for upgrades, matches original LoxBerry)
        if request.action == InstallAction::Upgrade {
            if let Some(result) = self
                .lifecycle_manager
                .execute_hook_with_args(
                    LifecycleHook::PostUpgrade,
                    &final_plugin_dir,
                    &config.plugin.folder,
                    &config.plugin.name,
                    &config.plugin.version,
                    Some(plugin_dir),
                )
                .await?
            {
                if !result.success {
                    warn!(
                        "PostUpgrade hook failed with exit code {:?}",
                        result.exit_code
                    );
                }
            }
        }

        // Execute postroot hook
        if let Some(result) = self
            .lifecycle_manager
            .execute_hook_with_args(
                LifecycleHook::PostRoot,
                &final_plugin_dir,
                &config.plugin.folder,
                &config.plugin.name,
                &config.plugin.version,
                Some(plugin_dir),
            )
            .await?
        {
            if !result.success {
                warn!("PostRoot hook failed with exit code {:?}", result.exit_code);
                // Don't fail installation on postroot failure, just warn
            }
        }

        // Parse autoupdate setting from AUTOUPDATE section or PLUGIN section
        // Original LoxBerry: AUTOUPDATE.AUTOMATIC_UPDATES=1 → autoupdate=3 (releases)
        let autoupdate = if let Some(ref au) = config.autoupdate {
            if au.automatic_updates.as_deref() == Some("1")
                || au.automatic_updates.as_deref() == Some("true")
            {
                // If enabling autoupdate and no existing setting, default to 3 (releases)
                existing
                    .and_then(|e| {
                        if e.autoupdate > 0 {
                            Some(e.autoupdate)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(3)
            } else {
                0
            }
        } else {
            config
                .plugin
                .autoupdate
                .as_ref()
                .and_then(|s| s.parse::<u8>().ok())
                .unwrap_or(0)
        };

        // Get release config URLs from AUTOUPDATE section or PLUGIN section
        let releasecfg = config
            .autoupdate
            .as_ref()
            .and_then(|au| au.releasecfg.clone())
            .or_else(|| config.plugin.releasecfg.clone());
        let prereleasecfg = config
            .autoupdate
            .as_ref()
            .and_then(|au| au.prereleasecfg.clone())
            .or_else(|| config.plugin.prereleasecfg.clone());

        // Get interface version from SYSTEM section (original LoxBerry location)
        let interface = config
            .system
            .as_ref()
            .and_then(|s| s.interface.clone())
            .or_else(|| config.plugin.interface.clone());

        // Get custom loglevels setting
        let loglevels_enabled = config
            .system
            .as_ref()
            .and_then(|s| s.custom_loglevels.clone());

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Build plugin system file paths (matches original LoxBerry PluginDB.pm save())
        let plugin_name = &config.plugin.name;
        let files = PluginFiles {
            daemon: Some(format!(
                "{}/system/daemons/plugins/{}",
                self.lbhomedir.display(),
                plugin_name
            )),
            uninstall: Some(format!(
                "{}/data/system/uninstall/{}",
                self.lbhomedir.display(),
                plugin_name
            )),
            sudoers: Some(format!(
                "{}/system/sudoers/{}",
                self.lbhomedir.display(),
                plugin_name
            )),
        };

        let install_files_dir = format!(
            "{}/data/system/install/{}",
            self.lbhomedir.display(),
            config.plugin.folder
        );

        // Create plugin entry for database
        let plugin_entry = PluginEntry {
            md5: md5.clone(),
            author_name: config.author.name.clone(),
            author_email: config.author.email.clone(),
            version: config.plugin.version.clone(),
            name: config.plugin.name.clone(),
            folder: config.plugin.folder.clone(),
            title: config.plugin.title.clone(),
            interface,
            autoupdate,
            releasecfg,
            prereleasecfg,
            loglevel: config
                .plugin
                .loglevel
                .clone()
                .unwrap_or_else(|| "3".to_string()),
            loglevels_enabled,
            directories: PluginDirectories {
                lbphtmlauthdir: paths.lbphtmlauthdir.clone(),
                lbphtmldir: paths.lbphtmldir.clone(),
                lbptemplatedir: paths.lbptemplatedir.clone(),
                lbpdatadir: paths.lbpdatadir.clone(),
                lbplogdir: paths.lbplogdir.clone(),
                lbpconfigdir: paths.lbpconfigdir.clone(),
                lbpbindir: paths.lbpbindir.clone(),
                installfiles: Some(install_files_dir),
            },
            files: Some(files),
            epoch_firstinstalled: match request.action {
                InstallAction::Install => Some(now),
                InstallAction::Upgrade | InstallAction::Reinstall => {
                    existing.and_then(|e| e.epoch_firstinstalled)
                }
            },
            epoch_lastupdated: Some(now),
            orig_name,
            orig_folder,
        };

        // Update database
        db.upsert(plugin_entry.clone());
        db.save(&self.db_path).await?;

        info!(
            "Successfully installed plugin: {} v{}",
            config.plugin.name, config.plugin.version
        );

        Ok(plugin_entry)
    }

    /// Uninstall a plugin
    ///
    /// Matches the original LoxBerry purge_installation("all") behavior:
    /// removes all plugin files, cron jobs, daemon files, sudoers, icons, etc.
    pub async fn uninstall(&self, md5: &str) -> Result<()> {
        info!("Uninstalling plugin with MD5: {}", md5);

        // Load plugin database
        let mut db = PluginDatabase::load(&self.db_path).await?;

        // Find plugin
        let plugin = db
            .find_by_md5(md5)
            .ok_or_else(|| Error::plugin("Plugin not found in database"))?;

        let folder = plugin.folder.clone();
        let name = plugin.name.clone();
        let version = plugin.version.clone();
        let plugin_dir = self.lbhomedir.join("bin/plugins").join(&folder);

        info!("Uninstalling plugin: {} ({})", name, folder);

        // Execute uninstall scripts from data/system/uninstall/{name}*
        let uninstall_dir = self.lbhomedir.join("data/system/uninstall");
        if uninstall_dir.exists() {
            for entry in WalkDir::new(&uninstall_dir)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let fname = entry.file_name().to_string_lossy();
                if fname.starts_with(&name) && entry.file_type().is_file() {
                    if let Some(result) = self
                        .lifecycle_manager
                        .execute_hook_with_args(
                            LifecycleHook::Uninstall,
                            entry.path().parent().unwrap_or(&uninstall_dir),
                            &folder,
                            &name,
                            &version,
                            None,
                        )
                        .await?
                    {
                        if !result.success {
                            warn!(
                                "Uninstall script {} failed with exit code {:?}",
                                fname, result.exit_code
                            );
                        }
                    }
                }
            }
        }

        // Fallback: execute uninstall hook from plugin bin directory
        if plugin_dir.exists() {
            if let Some(result) = self
                .lifecycle_manager
                .execute_hook_with_args(
                    LifecycleHook::Uninstall,
                    &plugin_dir,
                    &folder,
                    &name,
                    &version,
                    None,
                )
                .await?
            {
                if !result.success {
                    warn!(
                        "Uninstall hook failed with exit code {:?}",
                        result.exit_code
                    );
                }
            }
        }

        // Remove plugin directories (standard 7 directories)
        self.directory_manager
            .remove_plugin_structure(&folder)
            .await?;

        // Remove cron jobs: system/cron/*/{name}
        let cron_base = self.lbhomedir.join("system/cron");
        if cron_base.exists() {
            for entry in WalkDir::new(&cron_base)
                .min_depth(1)
                .max_depth(2)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let fname = entry.file_name().to_string_lossy();
                if fname == name || fname.starts_with(&format!("{}_", name)) {
                    if entry.file_type().is_dir() {
                        fs::remove_dir_all(entry.path()).await.ok();
                    } else {
                        fs::remove_file(entry.path()).await.ok();
                    }
                    debug!("Removed cron entry: {}", entry.path().display());
                }
            }
        }

        // Remove daemon files: system/daemons/plugins/{name}*
        let daemon_dir = self.lbhomedir.join("system/daemons/plugins");
        if daemon_dir.exists() {
            for entry in WalkDir::new(&daemon_dir)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_name().to_string_lossy().starts_with(&name) {
                    fs::remove_file(entry.path()).await.ok();
                    debug!("Removed daemon file: {}", entry.path().display());
                }
            }
        }

        // Remove sudoers file: system/sudoers/{name}
        let sudoers_file = self.lbhomedir.join(format!("system/sudoers/{}", name));
        if sudoers_file.exists() {
            fs::remove_file(&sudoers_file).await.ok();
            debug!("Removed sudoers file");
        }

        // Remove uninstall scripts: data/system/uninstall/{name}*
        if uninstall_dir.exists() {
            for entry in WalkDir::new(&uninstall_dir)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_name().to_string_lossy().starts_with(&name) {
                    fs::remove_file(entry.path()).await.ok();
                }
            }
        }

        // Remove icons: webfrontend/html/system/images/icons/{folder}
        let icons_dir = self
            .lbhomedir
            .join(format!("webfrontend/html/system/images/icons/{}", folder));
        if icons_dir.exists() {
            fs::remove_dir_all(&icons_dir).await.ok();
            debug!("Removed icons directory");
        }

        // Remove install backup: data/system/install/{folder}
        let install_dir = self
            .lbhomedir
            .join(format!("data/system/install/{}", folder));
        if install_dir.exists() {
            fs::remove_dir_all(&install_dir).await.ok();
            debug!("Removed install backup directory");
        }

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
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode)).ok();
                    // Ignore errors
                }
            }
        }

        info!("Extracted {} files to temp directory", archive.len());
        Ok(temp_dir)
    }

    /// Find plugin.cfg in extracted directory (may be in root or subdirectory)
    fn find_plugin_cfg(&self, base_dir: &Path) -> Result<PathBuf> {
        // First check root
        let root_cfg = base_dir.join("plugin.cfg");
        if root_cfg.exists() {
            return Ok(root_cfg);
        }

        // Search in subdirectories (max depth 2)
        for entry in WalkDir::new(base_dir)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_name() == "plugin.cfg" && entry.file_type().is_file() {
                return Ok(entry.path().to_path_buf());
            }
        }

        Err(Error::plugin(
            "plugin.cfg not found in archive (searched root and subdirectories)",
        ))
    }

    /// Copy plugin files to their destinations
    ///
    /// Matches the original LoxBerry plugininstall.pl file mapping
    async fn copy_plugin_files(&self, source_dir: &Path, config: &PluginConfig) -> Result<()> {
        let folder = &config.plugin.folder;
        let name = &config.plugin.name;
        info!("Copying plugin files for: {}", folder);

        // Standard directory mappings (matches original LoxBerry)
        let dir_mappings = vec![
            (
                "webfrontend/htmlauth",
                format!("webfrontend/htmlauth/plugins/{}", folder),
            ),
            (
                "webfrontend/html",
                format!("webfrontend/html/plugins/{}", folder),
            ),
            ("templates", format!("templates/plugins/{}", folder)),
            ("data", format!("data/plugins/{}", folder)),
            ("config", format!("config/plugins/{}", folder)),
            ("bin", format!("bin/plugins/{}", folder)),
        ];

        for (src_rel, dst_rel) in dir_mappings {
            let src = source_dir.join(src_rel);
            if !src.exists() {
                debug!("Skipping non-existent directory: {}", src_rel);
                continue;
            }

            let dst = self.lbhomedir.join(&dst_rel);
            fs::create_dir_all(&dst).await.map_err(|e| {
                Error::plugin(format!("Failed to create directory {}: {}", dst_rel, e))
            })?;

            self.copy_dir_recursive(&src, &dst).await?;
            info!("Copied {} -> {}", src_rel, dst_rel);
        }

        // Icons: icons/ → webfrontend/html/system/images/icons/{folder}/
        let icons_src = source_dir.join("icons");
        if icons_src.exists() {
            let icons_dst = self
                .lbhomedir
                .join(format!("webfrontend/html/system/images/icons/{}", folder));
            fs::create_dir_all(&icons_dst)
                .await
                .map_err(|e| Error::plugin(format!("Failed to create icons directory: {}", e)))?;
            self.copy_dir_recursive(&icons_src, &icons_dst).await?;
            info!("Copied icons/ -> images/icons/{}", folder);
        }

        // Cron jobs: cron/{interval}/ → system/cron/{interval}/{name}
        let cron_src = source_dir.join("cron");
        if cron_src.exists() {
            let cron_intervals = [
                "cron.reboot",
                "cron.01min",
                "cron.03min",
                "cron.05min",
                "cron.10min",
                "cron.15min",
                "cron.30min",
                "cron.hourly",
                "cron.daily",
                "cron.weekly",
                "cron.monthly",
                "cron.yearly",
            ];

            for interval in &cron_intervals {
                let interval_src = cron_src.join(interval);
                if interval_src.exists() {
                    let interval_dst = self
                        .lbhomedir
                        .join(format!("system/cron/{}/{}", interval, name));
                    fs::create_dir_all(&interval_dst).await.map_err(|e| {
                        Error::plugin(format!("Failed to create cron directory: {}", e))
                    })?;
                    self.copy_dir_recursive(&interval_src, &interval_dst)
                        .await?;
                    info!(
                        "Copied cron/{} -> system/cron/{}/{}",
                        interval, interval, name
                    );
                }
            }

            // cron/crontab → system/cron/cron.d/{name}
            let crontab_src = cron_src.join("crontab");
            if crontab_src.exists() {
                let crontab_dst = self.lbhomedir.join(format!("system/cron/cron.d/{}", name));
                if let Some(parent) = crontab_dst.parent() {
                    fs::create_dir_all(parent).await.map_err(|e| {
                        Error::plugin(format!("Failed to create cron.d directory: {}", e))
                    })?;
                }
                fs::copy(&crontab_src, &crontab_dst)
                    .await
                    .map_err(|e| Error::plugin(format!("Failed to copy crontab: {}", e)))?;
                info!("Copied cron/crontab -> system/cron/cron.d/{}", name);
            }
        }

        // Daemon files: daemon/daemon* → system/daemons/plugins/{name}
        let daemon_src = source_dir.join("daemon");
        if daemon_src.exists() {
            let daemon_dst_dir = self.lbhomedir.join("system/daemons/plugins");
            fs::create_dir_all(&daemon_dst_dir)
                .await
                .map_err(|e| Error::plugin(format!("Failed to create daemons directory: {}", e)))?;

            let mut daemon_idx = 0;
            for entry in WalkDir::new(&daemon_src)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file()
                    && entry.file_name().to_string_lossy().starts_with("daemon")
                {
                    let dst_name = if daemon_idx == 0 {
                        name.to_string()
                    } else {
                        format!("{}{}", name, daemon_idx - 1)
                    };
                    let dst = daemon_dst_dir.join(&dst_name);
                    fs::copy(entry.path(), &dst)
                        .await
                        .map_err(|e| Error::plugin(format!("Failed to copy daemon file: {}", e)))?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o755))
                            .await
                            .ok();
                    }
                    daemon_idx += 1;
                    info!("Copied daemon file -> system/daemons/plugins/{}", dst_name);
                }
            }
        }

        // Uninstall scripts: uninstall/uninstall* → data/system/uninstall/{name}
        let uninstall_src = source_dir.join("uninstall");
        if uninstall_src.exists() {
            let uninstall_dst_dir = self.lbhomedir.join("data/system/uninstall");
            fs::create_dir_all(&uninstall_dst_dir).await.map_err(|e| {
                Error::plugin(format!("Failed to create uninstall directory: {}", e))
            })?;

            let mut uninstall_idx = 0;
            for entry in WalkDir::new(&uninstall_src)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file()
                    && entry.file_name().to_string_lossy().starts_with("uninstall")
                {
                    let dst_name = if uninstall_idx == 0 {
                        name.to_string()
                    } else {
                        format!("{}{}", name, uninstall_idx - 1)
                    };
                    let dst = uninstall_dst_dir.join(&dst_name);
                    fs::copy(entry.path(), &dst).await.map_err(|e| {
                        Error::plugin(format!("Failed to copy uninstall script: {}", e))
                    })?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o755))
                            .await
                            .ok();
                    }
                    uninstall_idx += 1;
                    info!(
                        "Copied uninstall script -> data/system/uninstall/{}",
                        dst_name
                    );
                }
            }
        }

        // Sudoers: sudoers/sudoers → system/sudoers/{name}
        let sudoers_src = source_dir.join("sudoers/sudoers");
        if sudoers_src.exists() {
            let sudoers_dst_dir = self.lbhomedir.join("system/sudoers");
            fs::create_dir_all(&sudoers_dst_dir)
                .await
                .map_err(|e| Error::plugin(format!("Failed to create sudoers directory: {}", e)))?;
            let sudoers_dst = sudoers_dst_dir.join(name);
            fs::copy(&sudoers_src, &sudoers_dst)
                .await
                .map_err(|e| Error::plugin(format!("Failed to copy sudoers file: {}", e)))?;
            info!("Copied sudoers/sudoers -> system/sudoers/{}", name);
        }

        // Backup install scripts to data/system/install/{folder}/
        let install_backup_dir = self
            .lbhomedir
            .join(format!("data/system/install/{}", folder));
        fs::create_dir_all(&install_backup_dir).await.map_err(|e| {
            Error::plugin(format!("Failed to create install backup directory: {}", e))
        })?;

        // Copy pre*/post* scripts and apt/dpkg dirs for reference
        for script_name in &[
            "preroot.sh",
            "preinstall.sh",
            "postinstall.sh",
            "postroot.sh",
            "preupgrade.sh",
            "postupgrade.sh",
        ] {
            let script_src = source_dir.join(script_name);
            if script_src.exists() {
                let script_dst = install_backup_dir.join(script_name);
                fs::copy(&script_src, &script_dst).await.ok();
            }
        }
        // Copy apt/dpkg directories for reference
        for dir_name in &["apt", "dpkg"] {
            let dir_src = source_dir.join(dir_name);
            if dir_src.exists() {
                let dir_dst = install_backup_dir.join(dir_name);
                fs::create_dir_all(&dir_dst).await.ok();
                self.copy_dir_recursive(&dir_src, &dir_dst).await.ok();
            }
        }

        // Perform REPLACELB* variable substitution in text files
        self.replace_variables(folder).await;

        Ok(())
    }

    /// Perform REPLACELB* variable substitution in plugin text files
    ///
    /// Matches the original LoxBerry plugininstall.pl behavior
    async fn replace_variables(&self, folder: &str) {
        let replacements = vec![
            ("REPLACELBHOMEDIR", self.lbhomedir.display().to_string()),
            ("REPLACELBPPLUGINDIR", folder.to_string()),
            (
                "REPLACELBPHTMLAUTHDIR",
                format!(
                    "{}/webfrontend/htmlauth/plugins/{}",
                    self.lbhomedir.display(),
                    folder
                ),
            ),
            (
                "REPLACELBPHTMLDIR",
                format!(
                    "{}/webfrontend/html/plugins/{}",
                    self.lbhomedir.display(),
                    folder
                ),
            ),
            (
                "REPLACELBPTEMPLATEDIR",
                format!("{}/templates/plugins/{}", self.lbhomedir.display(), folder),
            ),
            (
                "REPLACELBPDATADIR",
                format!("{}/data/plugins/{}", self.lbhomedir.display(), folder),
            ),
            (
                "REPLACELBPLOGDIR",
                format!("{}/log/plugins/{}", self.lbhomedir.display(), folder),
            ),
            (
                "REPLACELBPCONFIGDIR",
                format!("{}/config/plugins/{}", self.lbhomedir.display(), folder),
            ),
            (
                "REPLACELBPBINDIR",
                format!("{}/bin/plugins/{}", self.lbhomedir.display(), folder),
            ),
        ];

        // Walk all plugin directories and replace in text files
        let plugin_dirs = vec![
            format!("webfrontend/htmlauth/plugins/{}", folder),
            format!("webfrontend/html/plugins/{}", folder),
            format!("templates/plugins/{}", folder),
            format!("data/plugins/{}", folder),
            format!("config/plugins/{}", folder),
            format!("bin/plugins/{}", folder),
        ];

        for dir_rel in plugin_dirs {
            let dir = self.lbhomedir.join(&dir_rel);
            if !dir.exists() {
                continue;
            }

            for entry in WalkDir::new(&dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();

                // Skip binary files by checking extension
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let binary_exts = [
                    "png", "jpg", "jpeg", "gif", "ico", "svg", "woff", "woff2", "ttf", "eot",
                    "zip", "gz", "tar", "bz2", "xz", "pdf", "bin", "so", "o", "pyc",
                ];
                if binary_exts.contains(&ext.as_str()) {
                    continue;
                }

                // Read file, perform replacements + DOS2UNIX, write back
                if let Ok(content) = tokio::fs::read_to_string(path).await {
                    let mut modified = content.clone();
                    // REPLACELB* variable substitution
                    for (placeholder, value) in &replacements {
                        modified = modified.replace(placeholder, value);
                    }
                    // DOS to Unix line ending conversion (\r\n → \n)
                    if modified.contains("\r\n") {
                        modified = modified.replace("\r\n", "\n");
                    }
                    if modified != content {
                        tokio::fs::write(path, &modified).await.ok();
                        debug!("Processed text file: {}", path.display());
                    }
                }
            }
        }
    }

    /// Install APT packages from dpkg/apt file
    ///
    /// Matches original LoxBerry: reads package list from dpkg/apt (or version-specific
    /// files like dpkg/apt12 for Debian 12), then runs apt-get install.
    /// Non-fatal: warnings are logged but installation continues.
    async fn install_apt_packages(&self, source_dir: &Path) {
        let dpkg_dir = source_dir.join("dpkg");
        if !dpkg_dir.exists() {
            return;
        }

        // Try version-specific apt files first, then fall back to generic
        // Priority: apt{debian_major} > apt (generic)
        let apt_file = self.find_apt_file(&dpkg_dir).await;
        let apt_file = match apt_file {
            Some(f) => f,
            None => return,
        };

        // Read and parse the apt file
        let content = match tokio::fs::read_to_string(&apt_file).await {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to read APT file {}: {}", apt_file.display(), e);
                return;
            }
        };

        let packages: Vec<String> = content
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.to_string())
            .collect();

        if packages.is_empty() {
            return;
        }

        info!("Installing APT packages: {:?}", packages);

        // Run apt-get update first
        let update_result = tokio::process::Command::new("apt-get")
            .args(["update", "-q"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
            .await;

        if let Err(e) = &update_result {
            warn!("apt-get update failed: {} (continuing anyway)", e);
        }

        // Install packages
        let install_result = tokio::process::Command::new("apt-get")
            .args(["install", "-y", "--no-install-recommends"])
            .args(&packages)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await;

        match install_result {
            Ok(output) => {
                if output.status.success() {
                    info!("APT packages installed successfully");
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    warn!("APT package installation had issues: {}", stderr);
                }
            }
            Err(e) => {
                warn!("Failed to run apt-get install: {}", e);
            }
        }

        // Install architecture-specific .deb files
        let arch = std::env::consts::ARCH;
        let deb_arch = match arch {
            "x86_64" => "amd64",
            "aarch64" => "arm64",
            "arm" => "armhf",
            _ => arch,
        };

        let arch_deb_dir = dpkg_dir.join(deb_arch);
        if arch_deb_dir.exists() {
            info!("Installing .deb packages from {}", arch_deb_dir.display());
            let dpkg_result = tokio::process::Command::new("dpkg")
                .args(["-i", "-R"])
                .arg(&arch_deb_dir)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
                .await;

            match dpkg_result {
                Ok(output) if !output.status.success() => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    warn!("dpkg install had issues: {}", stderr);
                }
                Err(e) => warn!("Failed to run dpkg: {}", e),
                _ => info!("Architecture .deb packages installed"),
            }
        }
    }

    /// Find the best matching APT file for this system
    async fn find_apt_file(&self, dpkg_dir: &Path) -> Option<PathBuf> {
        // Try to detect Debian major version
        if let Ok(content) = tokio::fs::read_to_string("/etc/os-release").await {
            for line in content.lines() {
                if let Some(version) = line.strip_prefix("VERSION_ID=") {
                    let version = version.trim_matches('"');
                    if let Some(major) = version.split('.').next() {
                        let versioned = dpkg_dir.join(format!("apt{}", major));
                        if versioned.exists() {
                            return Some(versioned);
                        }
                    }
                }
            }
        }

        // Fall back to generic apt file
        let generic = dpkg_dir.join("apt");
        if generic.exists() {
            return Some(generic);
        }

        None
    }

    /// Check if the system supports the required architecture
    fn check_architecture(&self, arch_list: &str) -> bool {
        let system_arch = std::env::consts::ARCH;
        let config_dir = self.lbhomedir.join("config/system");

        for arch in arch_list.split(',') {
            let arch = arch.trim().to_lowercase();
            if arch.is_empty() || arch == "all" {
                return true;
            }

            // Check via config files (matches original LoxBerry: is_$arch.cfg or is_arch_$arch.cfg)
            let is_arch_file = config_dir.join(format!("is_{}.cfg", arch));
            let is_arch_file2 = config_dir.join(format!("is_arch_{}.cfg", arch));
            if is_arch_file.exists() || is_arch_file2.exists() {
                return true;
            }

            // Also check against Rust's detected architecture
            let matches = match arch.as_str() {
                "x86_64" | "x64" | "amd64" => system_arch == "x86_64",
                "aarch64" | "arm64" => system_arch == "aarch64",
                "armhf" | "armv7l" | "arm" => system_arch == "arm",
                "i386" | "i686" => system_arch == "x86",
                // "x86" in LoxBerry means the x86 PC platform (both 32-bit and 64-bit)
                "x86" => system_arch == "x86" || system_arch == "x86_64",
                "raspberry" => system_arch == "arm" || system_arch == "aarch64",
                _ => arch == system_arch,
            };
            if matches {
                return true;
            }
        }

        false
    }

    /// Get the system version from general.json
    async fn get_system_version(&self) -> Option<String> {
        let config_path = self.lbhomedir.join("config/system/general.json");
        if let Ok(content) = tokio::fs::read_to_string(&config_path).await {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                return json["Base"]["Version"].as_str().map(|s| s.to_string());
            }
        }
        None
    }

    /// Recursively copy directory contents
    async fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> Result<()> {
        for entry in WalkDir::new(src).min_depth(1) {
            let entry =
                entry.map_err(|e| Error::plugin(format!("Failed to walk directory: {}", e)))?;

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

/// Sanitize a plugin name or folder to [A-Za-z0-9_-] only
/// Matches original LoxBerry: `$pname =~ s/[^A-Za-z0-9_-]//g`
fn sanitize_identifier(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

/// Simple version comparison (returns -1, 0, or 1)
/// Compares version strings like "3.0.0" vs "4.0.0.0"
fn version_compare(a: &str, b: &str) -> i32 {
    let parse_parts = |s: &str| -> Vec<u64> {
        s.trim_start_matches('v')
            .split('.')
            .filter_map(|p| p.parse().ok())
            .collect()
    };

    let a_parts = parse_parts(a);
    let b_parts = parse_parts(b);
    let max_len = a_parts.len().max(b_parts.len());

    for i in 0..max_len {
        let av = a_parts.get(i).copied().unwrap_or(0);
        let bv = b_parts.get(i).copied().unwrap_or(0);
        match av.cmp(&bv) {
            std::cmp::Ordering::Less => return -1,
            std::cmp::Ordering::Greater => return 1,
            std::cmp::Ordering::Equal => continue,
        }
    }
    0
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
