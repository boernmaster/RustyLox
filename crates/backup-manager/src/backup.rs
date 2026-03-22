//! Backup creation

use chrono::{DateTime, Utc};
use rustylox_core::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub loxberry_version: String,
    pub includes: Vec<String>,
    pub size_bytes: u64,
}

pub struct BackupManager {
    lbhomedir: PathBuf,
}

impl BackupManager {
    pub fn new(lbhomedir: PathBuf) -> Self {
        Self { lbhomedir }
    }

    /// Create a new backup
    pub async fn create_backup(&self, include_plugins: bool) -> Result<PathBuf> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_name = format!("loxberry_backup_{}.tar.gz", timestamp);
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
            loxberry_version: "4.0.0.0".to_string(),
            includes: includes.clone(),
            size_bytes: 0, // Will be updated after creation
        };

        // Create tar.gz
        let tar_file = File::create(&backup_path)?;
        let enc = flate2::write::GzEncoder::new(tar_file, flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);

        // Add metadata.json
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        let mut header = tar::Header::new_gnu();
        header.set_path("metadata.json")?;
        header.set_size(metadata_json.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append(&header, metadata_json.as_bytes())?;

        // Add directories
        for include in &includes {
            let source = self.lbhomedir.join(include);
            if source.exists() {
                tracing::info!("Adding to backup: {}", include);
                tar.append_dir_all(include, &source)?;
            } else {
                tracing::warn!("Backup source not found: {}", include);
            }
        }

        tar.finish()?;

        // Update metadata with actual size
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

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("gz") {
                let metadata = tokio::fs::metadata(&path).await?;
                let name = path.file_name().unwrap().to_string_lossy().to_string();

                // Try to extract metadata from backup
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
        let tar_file = File::open(backup_path)?;
        let dec = flate2::read::GzDecoder::new(tar_file);
        let mut archive = tar::Archive::new(dec);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;

            if path == PathBuf::from("metadata.json") {
                let mut contents = String::new();
                std::io::Read::read_to_string(&mut entry, &mut contents)?;
                return Ok(serde_json::from_str(&contents)?);
            }
        }

        Err(rustylox_core::Error::backup(
            "Metadata not found in backup".to_string(),
        ))
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
pub async fn create_backup(lbhomedir: PathBuf, include_plugins: bool) -> Result<PathBuf> {
    let manager = BackupManager::new(lbhomedir);
    manager.create_backup(include_plugins).await
}
