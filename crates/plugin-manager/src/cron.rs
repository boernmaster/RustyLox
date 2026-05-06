//! Plugin cron scheduler — executes scripts placed by plugins into
//! `$LBHOMEDIR/system/cron/<interval>/` on the corresponding schedule.
//!
//! LoxBerry-compatible cron directories and their intervals:
//! - `cron.01min`  — every 1 minute
//! - `cron.03min`  — every 3 minutes
//! - `cron.05min`  — every 5 minutes
//! - `cron.10min`  — every 10 minutes
//! - `cron.15min`  — every 15 minutes
//! - `cron.30min`  — every 30 minutes
//! - `cron.hourly` — every hour
//! - `cron.daily`  — every day
//! - `cron.weekly` — every week
//! - `cron.monthly`— every 30 days

use std::path::{Path, PathBuf};
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

/// Mapping of cron directory name → period in seconds.
const CRON_SCHEDULES: &[(&str, u64)] = &[
    ("cron.01min", 60),
    ("cron.03min", 3 * 60),
    ("cron.05min", 5 * 60),
    ("cron.10min", 10 * 60),
    ("cron.15min", 15 * 60),
    ("cron.30min", 30 * 60),
    ("cron.hourly", 3600),
    ("cron.daily", 86400),
    ("cron.weekly", 7 * 86400),
    ("cron.monthly", 30 * 86400),
];

/// Timeout applied to each individual cron script execution (5 minutes).
const SCRIPT_TIMEOUT_SECS: u64 = 300;

/// Spawn one background Tokio task per cron interval.  Each task wakes at
/// its interval, scans the matching `system/cron/<dir>/` directory for
/// executable files, and runs every executable it finds (each in its own
/// spawned sub-task so they execute concurrently and cannot block each other).
///
/// The function itself returns immediately after spawning the tasks.
pub async fn run_plugin_cron_schedules(lbhomedir: PathBuf) {
    info!(
        "Starting plugin cron scheduler (home={})",
        lbhomedir.display()
    );

    for (dir_name, period_secs) in CRON_SCHEDULES {
        let home = lbhomedir.clone();
        let dir = dir_name.to_string();
        let period = *period_secs;

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(period));
            // Skip the immediate first tick so scripts don't fire on startup.
            ticker.tick().await;

            loop {
                ticker.tick().await;

                let cron_dir = home.join("system/cron").join(&dir);

                if !cron_dir.exists() {
                    // Directory absent — nothing to do for this tick.
                    continue;
                }

                let entries = match tokio::fs::read_dir(&cron_dir).await {
                    Ok(e) => e,
                    Err(err) => {
                        warn!(
                            "Failed to read cron directory {}: {}",
                            cron_dir.display(),
                            err
                        );
                        continue;
                    }
                };

                run_scripts_in_dir(entries, &home, &dir).await;
            }
        });
    }
}

/// Iterate over directory entries and spawn a task for each executable file.
async fn run_scripts_in_dir(mut entries: tokio::fs::ReadDir, home: &Path, dir_label: &str) {
    loop {
        let entry = match entries.next_entry().await {
            Ok(Some(e)) => e,
            Ok(None) => break,
            Err(err) => {
                warn!("Error reading cron dir entry ({}): {}", dir_label, err);
                break;
            }
        };

        let path = entry.path();

        // Skip non-files (directories, symlinks to dirs, etc.)
        let metadata = match tokio::fs::metadata(&path).await {
            Ok(m) => m,
            Err(err) => {
                warn!("Cannot stat {}: {}", path.display(), err);
                continue;
            }
        };

        if !metadata.is_file() {
            continue;
        }

        // On Unix, only run files that have at least one executable bit set.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o111 == 0 {
                debug!("Skipping non-executable cron script: {}", path.display());
                continue;
            }
        }

        let script = path.clone();
        let home_clone = home.to_path_buf();

        tokio::spawn(async move {
            execute_cron_script(script, home_clone).await;
        });
    }
}

/// Execute a single cron script with a 5-minute timeout and a basic set of
/// LoxBerry environment variables.
async fn execute_cron_script(script: PathBuf, lbhomedir: PathBuf) {
    debug!("Running cron script: {}", script.display());

    let plugindatabase = format!("{}/data/system/plugindatabase.json", lbhomedir.display());
    let perl5lib = format!("{}/libs/perllib", lbhomedir.display());

    let result = tokio::time::timeout(
        Duration::from_secs(SCRIPT_TIMEOUT_SECS),
        tokio::process::Command::new(&script)
            .env("LBHOMEDIR", lbhomedir.to_string_lossy().as_ref())
            .env("PLUGINDATABASE", &plugindatabase)
            .env("PERL5LIB", &perl5lib)
            .env("LBSVERSION", "4.0.0.0")
            .current_dir(&lbhomedir)
            .output(),
    )
    .await;

    match result {
        Ok(Ok(out)) if out.status.success() => {
            debug!("Cron script completed successfully: {}", script.display());
        }
        Ok(Ok(out)) => {
            warn!(
                "Cron script {} exited with status {}: {}",
                script.display(),
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(Err(err)) => {
            error!("Failed to spawn cron script {}: {}", script.display(), err);
        }
        Err(_) => {
            warn!(
                "Cron script timed out after {}s: {}",
                SCRIPT_TIMEOUT_SECS,
                script.display()
            );
        }
    }
}
