//! Backup restoration

use loxberry_core::Result;
use std::fs::File;
use std::path::PathBuf;

/// Restore from a backup file
pub async fn restore_backup(lbhomedir: PathBuf, backup_path: PathBuf) -> Result<()> {
    if !backup_path.exists() {
        return Err(loxberry_core::Error::backup(
            "Backup file not found".to_string(),
        ));
    }

    tracing::info!("Restoring from backup: {}", backup_path.display());

    // Extract tar.gz
    let tar_file = File::open(&backup_path)?;
    let dec = flate2::read::GzDecoder::new(tar_file);
    let mut archive = tar::Archive::new(dec);

    // Extract to lbhomedir
    archive.unpack(&lbhomedir)?;

    tracing::info!("Backup restored successfully");

    Ok(())
}
