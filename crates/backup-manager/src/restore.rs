//! Backup restoration

use rustylox_core::Result;
use std::fs::File;
use std::path::PathBuf;

/// Restore from a backup file
pub async fn restore_backup(lbhomedir: PathBuf, backup_path: PathBuf) -> Result<()> {
    if !backup_path.exists() {
        return Err(rustylox_core::Error::backup(
            "Backup file not found".to_string(),
        ));
    }

    tracing::info!("Restoring from backup: {}", backup_path.display());

    let file = File::open(&backup_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    archive.extract(&lbhomedir)?;

    tracing::info!("Backup restored successfully");

    Ok(())
}
