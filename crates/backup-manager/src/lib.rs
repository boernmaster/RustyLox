//! Backup and restore functionality for LoxBerry
//!
//! This crate provides:
//! - Configuration backup
//! - Plugin data backup
//! - Restore from backup
//! - Scheduled backups

pub mod backup;
pub mod restore;
pub mod scheduler;

pub use backup::{create_backup, BackupManager, BackupMetadata};
pub use restore::restore_backup;
pub use scheduler::BackupScheduler;

use std::path::{Path, PathBuf};

/// Backup format version
pub const BACKUP_VERSION: &str = "1.0.0";

/// Get backup directory
pub fn backup_dir(lbhomedir: &Path) -> PathBuf {
    lbhomedir.join("data/backup")
}
