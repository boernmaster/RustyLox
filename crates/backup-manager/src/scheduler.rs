//! Scheduled backup functionality

use rustylox_core::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::time::{interval, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSchedule {
    pub enabled: bool,
    pub interval_hours: u64,
    pub include_plugins: bool,
    pub max_backups: usize,
}

impl Default for BackupSchedule {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_hours: 24,
            include_plugins: true,
            max_backups: 7,
        }
    }
}

pub struct BackupScheduler {
    lbhomedir: PathBuf,
    schedule: BackupSchedule,
}

impl BackupScheduler {
    pub fn new(lbhomedir: PathBuf, schedule: BackupSchedule) -> Self {
        Self {
            lbhomedir,
            schedule,
        }
    }

    /// Start the scheduler
    pub async fn run(&self) -> Result<()> {
        if !self.schedule.enabled {
            tracing::info!("Backup scheduler is disabled");
            return Ok(());
        }

        tracing::info!(
            "Starting backup scheduler: every {} hours",
            self.schedule.interval_hours
        );

        let mut interval = interval(Duration::from_secs(self.schedule.interval_hours * 3600));

        loop {
            interval.tick().await;

            tracing::info!("Running scheduled backup");

            match crate::backup::create_backup(
                self.lbhomedir.clone(),
                self.schedule.include_plugins,
            )
            .await
            {
                Ok(path) => {
                    tracing::info!("Scheduled backup created: {}", path.display());

                    // Clean up old backups
                    if let Err(e) = self.cleanup_old_backups().await {
                        tracing::error!("Failed to cleanup old backups: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Scheduled backup failed: {}", e);
                }
            }
        }
    }

    /// Remove old backups keeping only max_backups
    async fn cleanup_old_backups(&self) -> Result<()> {
        let manager = crate::backup::BackupManager::new(self.lbhomedir.clone());
        let mut backups = manager.list_backups().await?;

        if backups.len() <= self.schedule.max_backups {
            return Ok(());
        }

        // Sort by creation time (oldest first)
        backups.sort_by(|a, b| a.created.cmp(&b.created));

        // Remove oldest backups
        let to_remove = backups.len() - self.schedule.max_backups;
        for backup in backups.iter().take(to_remove) {
            tracing::info!("Removing old backup: {}", backup.name);
            manager.delete_backup(&backup.name).await?;
        }

        Ok(())
    }
}
