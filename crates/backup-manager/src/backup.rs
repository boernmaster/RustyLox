//! Backup creation

use chrono::{DateTime, Utc};
use rustylox_core::Result;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub rustylox_version: String,
    pub includes: Vec<String>,
    pub size_bytes: u64,
}

pub struct BackupManager {
    lbhomedir: PathBuf,
    version: String,
}

impl BackupManager {
    pub fn new(lbhomedir: PathBuf, version: String) -> Self {
        Self { lbhomedir, version }
    }

    /// Create a new backup
    pub async fn create_backup(&self, include_plugins: bool) -> Result<PathBuf> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_name = format!("rustylox_backup_{}.zip", timestamp);
        let backup_dir = crate::backup_dir(&self.lbhomedir);

        tokio::fs::create_dir_all(&backup_dir).await?;

        let backup_path = backup_dir.join(&backup_name);

        tracing::info!("Creating backup: {}", backup_path.display());

        // Create metadata
        let mut includes = vec!["config/system".to_string(), "data/system".to_string()];

        if include_plugins {
            includes.push("config/plugins".to_string());
            includes.push("data/plugins".to_string());
        }

        let metadata = BackupMetadata {
            version: crate::BACKUP_VERSION.to_string(),
            timestamp: Utc::now(),
            rustylox_version: self.version.clone(),
            includes: includes.clone(),
            size_bytes: 0,
        };

        // Create ZIP
        let zip_file = std::fs::File::create(&backup_path)?;
        let mut zip = zip::ZipWriter::new(zip_file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // Add metadata.json
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        zip.start_file("metadata.json", options)?;
        zip.write_all(metadata_json.as_bytes())?;

        // Add directories
        for include in &includes {
            let source = self.lbhomedir.join(include);
            if source.exists() {
                tracing::info!("Adding to backup: {}", include);
                for entry in WalkDir::new(&source).into_iter().filter_map(|e| e.ok()) {
                    let path = entry.path();
                    let rel_path = path.strip_prefix(&self.lbhomedir).unwrap_or(path);
                    let name = rel_path.to_string_lossy();

                    if path.is_dir() {
                        zip.add_directory(format!("{}/", name), options)?;
                    } else if path.is_file() {
                        zip.start_file(name.to_string(), options)?;
                        let data = std::fs::read(path)?;
                        zip.write_all(&data)?;
                    }
                }
            } else {
                tracing::warn!("Backup source not found: {}", include);
            }
        }

        zip.finish()?;

        let file_size = tokio::fs::metadata(&backup_path).await?.len();
        tracing::info!(
            "Backup created successfully: {} ({} bytes)",
            backup_path.display(),
            file_size
        );

        Ok(backup_path)
    }

    /// List all backups
    pub async fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        let backup_dir = crate::backup_dir(&self.lbhomedir);
        let mut backups = Vec::new();

        if !backup_dir.exists() {
            return Ok(backups);
        }

        let mut entries = tokio::fs::read_dir(&backup_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("zip") {
                let metadata = tokio::fs::metadata(&path).await?;
                let name = path.file_name().unwrap().to_string_lossy().to_string();

                let backup_metadata = self.read_backup_metadata(&path).ok();

                backups.push(BackupInfo {
                    name,
                    path,
                    size_bytes: metadata.len(),
                    created: metadata.modified()?.into(),
                    metadata: backup_metadata,
                });
            }
        }

        backups.sort_by(|a, b| b.created.cmp(&a.created));
        Ok(backups)
    }

    /// Read metadata from backup file
    fn read_backup_metadata(&self, backup_path: &PathBuf) -> Result<BackupMetadata> {
        let file = std::fs::File::open(backup_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let mut entry = archive.by_name("metadata.json").map_err(|e| {
            rustylox_core::Error::backup(format!("Metadata not found in backup: {}", e))
        })?;

        let mut contents = String::new();
        std::io::Read::read_to_string(&mut entry, &mut contents)?;
        Ok(serde_json::from_str(&contents)?)
    }

    /// Delete a backup
    pub async fn delete_backup(&self, backup_name: &str) -> Result<()> {
        let backup_dir = crate::backup_dir(&self.lbhomedir);
        let backup_path = backup_dir.join(backup_name);

        if !backup_path.exists() {
            return Err(rustylox_core::Error::backup(format!(
                "Backup not found: {}",
                backup_name
            )));
        }

        tokio::fs::remove_file(&backup_path).await?;
        tracing::info!("Deleted backup: {}", backup_name);

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupInfo {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub created: DateTime<Utc>,
    pub metadata: Option<BackupMetadata>,
}

/// Create a backup (convenience function)
pub async fn create_backup(
    lbhomedir: PathBuf,
    version: String,
    include_plugins: bool,
) -> Result<PathBuf> {
    let manager = BackupManager::new(lbhomedir, version);
    manager.create_backup(include_plugins).await
}
