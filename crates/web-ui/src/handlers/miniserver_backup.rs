//! Miniserver backup handlers
//!
//! Downloads the Loxone project file from the Miniserver using its file-system API:
//!   GET /dev/fslist/prog/   — list files in the /prog directory
//!   GET /dev/fsget/prog/<f> — download a specific file
//!
//! Backups are stored under `$LBHOMEDIR/data/system/miniserver-backups/<id>-<name>/`.

use askama::Template;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    Form,
};
use chrono::{Local, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, error, info, warn};
use web_api::AppState;

const BACKUPS_TO_KEEP: usize = 7;

// ---------------------------------------------------------------------------
// Schedule config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsBackupSchedule {
    pub enabled: bool,
    /// How often to run the backup (in hours)
    pub interval_hours: u32,
    /// Unix timestamp of the last successful backup
    pub last_run_ts: Option<i64>,
}

impl Default for MsBackupSchedule {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_hours: 24,
            last_run_ts: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MsBackupSchedules {
    pub schedules: HashMap<String, MsBackupSchedule>,
}

// ---------------------------------------------------------------------------
// Dedicated log file: log/system/miniserver-backup.log
// ---------------------------------------------------------------------------

async fn log_backup(lbhomedir: &Path, level: &str, message: &str) {
    use tokio::io::AsyncWriteExt;

    let log_path = lbhomedir.join("log/system/miniserver-backup.log");

    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent).await;
    }

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!(
        "[{}] [{}] [miniserver-backup] {}\n",
        timestamp, level, message
    );

    if let Ok(mut file) = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .await
    {
        let _ = file.write_all(line.as_bytes()).await;
    }
}

async fn load_ms_schedules(lbhomedir: &Path) -> MsBackupSchedules {
    let path = lbhomedir.join("config/system/ms_backup_schedule.json");
    let Ok(content) = fs::read_to_string(&path).await else {
        return MsBackupSchedules::default();
    };
    serde_json::from_str(&content).unwrap_or_default()
}

async fn save_ms_schedules(lbhomedir: &Path, schedules: &MsBackupSchedules) -> std::io::Result<()> {
    let path = lbhomedir.join("config/system/ms_backup_schedule.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let content = serde_json::to_string_pretty(schedules).map_err(std::io::Error::other)?;
    fs::write(&path, content).await
}

// ---------------------------------------------------------------------------
// Template types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct BackupFileDisplay {
    pub filename: String,
    pub size_human: String,
    pub created: String,
}

#[derive(Debug, Clone)]
pub struct MiniserverBackupEntry {
    pub id: String,
    pub name: String,
    pub ipaddress: String,
    pub backups: Vec<BackupFileDisplay>,
    pub schedule_enabled: bool,
    pub schedule_interval_hours: u32,
    pub schedule_last_run: String,
}

#[derive(Template)]
#[template(path = "miniserver/backup.html")]
pub struct MiniserverBackupTemplate {
    pub entries: Vec<MiniserverBackupEntry>,
    pub version: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Directory for a specific Miniserver's backups: `<base>/<id>-<safe-name>/`
fn ms_backup_dir(base: &Path, id: &str, name: &str) -> PathBuf {
    let safe_name: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    base.join(format!("{}-{}", id, safe_name))
}

/// Read existing backups for one Miniserver, newest first.
async fn list_backups_for(dir: &Path) -> Vec<BackupFileDisplay> {
    let mut entries = Vec::new();

    let Ok(mut rd) = fs::read_dir(dir).await else {
        return entries;
    };

    while let Ok(Some(entry)) = rd.next_entry().await {
        let path = entry.path();
        // Accept .loxone project files and any other backup files
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "loxone" | "zip") {
            continue;
        }
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let (size_human, created) = match fs::metadata(&path).await {
            Ok(meta) => {
                let size = format_size(meta.len());
                let ts = meta
                    .modified()
                    .ok()
                    .map(|t| {
                        let dt: chrono::DateTime<Local> = t.into();
                        dt.format("%Y-%m-%d %H:%M:%S").to_string()
                    })
                    .unwrap_or_else(|| "unknown".to_string());
                (size, ts)
            }
            Err(_) => ("?".to_string(), "unknown".to_string()),
        };
        entries.push(BackupFileDisplay {
            filename,
            size_human,
            created,
        });
    }

    // Sort newest first (lexicographic on timestamp-embedded filenames)
    entries.sort_by(|a, b| b.filename.cmp(&a.filename));
    entries
}

/// Delete oldest backups, keeping only `BACKUPS_TO_KEEP` most recent.
async fn rotate_backups(dir: &Path) {
    let mut files: Vec<PathBuf> = Vec::new();

    let Ok(mut rd) = fs::read_dir(dir).await else {
        return;
    };
    while let Ok(Some(entry)) = rd.next_entry().await {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if matches!(ext, "loxone" | "zip") {
            files.push(path);
        }
    }

    if files.len() <= BACKUPS_TO_KEEP {
        return;
    }

    // Sort oldest first (ascending) then remove the head
    files.sort();
    let to_delete = files.len() - BACKUPS_TO_KEEP;
    for path in files.iter().take(to_delete) {
        if let Err(e) = fs::remove_file(path).await {
            warn!("Failed to delete old backup {:?}: {}", path, e);
        } else {
            info!("Rotated old backup: {:?}", path);
        }
    }
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

/// Show the Miniserver backup management page.
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let base_dir = state.lbhomedir.join("data/system/miniserver-backups");

    let ms_entries: Vec<(String, String, String)> = {
        let config = state.config.read().await;
        config
            .miniserver
            .iter()
            .map(|(id, ms)| (id.clone(), ms.name.clone(), ms.ipaddress.clone()))
            .collect()
    };

    let schedules = load_ms_schedules(&state.lbhomedir).await;

    let mut entries = Vec::new();
    for (id, name, ipaddress) in ms_entries {
        let dir = ms_backup_dir(&base_dir, &id, &name);
        let backups = list_backups_for(&dir).await;
        let sched = schedules.schedules.get(&id).cloned().unwrap_or_default();
        let schedule_last_run = sched
            .last_run_ts
            .and_then(|ts| Utc.timestamp_opt(ts, 0).single())
            .map(|d| d.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Never".to_string());
        entries.push(MiniserverBackupEntry {
            id,
            name,
            ipaddress,
            backups,
            schedule_enabled: sched.enabled,
            schedule_interval_hours: sched.interval_hours,
            schedule_last_run,
        });
    }

    let template = MiniserverBackupTemplate {
        entries,
        version: state.version.clone(),
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// Parse the /dev/fslist response to find .loxone project filenames.
///
/// Accepts various line formats:
///   `- SIZE Mon DD HH:MM filename`  (standard format)
///   `d SIZE Mon DD HH:MM dirname`   (directory – skipped)
///   `filename.loxone`               (bare name format)
/// The last whitespace-separated token on each non-directory line is checked.
fn parse_loxone_files(listing: &str) -> Vec<String> {
    listing
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            if line.starts_with("d ") {
                return None;
            }
            let name = line.split_whitespace().last()?;
            if name.to_lowercase().ends_with(".loxone") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Parse the /dev/fslist response to return ALL filenames (any extension).
///
/// Used for full binary backups: collects every file in the directory,
/// including `.Loxone` project files and `permissions.bin`.
/// Directory entries (lines starting with `d `) are skipped.
fn parse_all_files(listing: &str) -> Vec<String> {
    listing
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            // Skip directory entries
            if line.starts_with("d ") {
                return None;
            }
            // The filename is the last whitespace-separated token
            line.split_whitespace().last().map(|s| s.to_string())
        })
        .collect()
}

/// Trigger a backup for one Miniserver (HTMX endpoint).
///
/// Uses the Miniserver file-system HTTP API:
///   GET /dev/fslist/prog/   → list project files
///   GET /dev/fsget/prog/<f> → download the project file
pub async fn run_backup(State(state): State<AppState>, Path(id): Path<String>) -> Html<String> {
    let ms_id: u8 = match id.parse() {
        Ok(n) => n,
        Err(_) => {
            return Html("<div class='alert alert-danger'>Invalid Miniserver ID</div>".to_string())
        }
    };

    let (ms_name, ms_ip) = {
        let config = state.config.read().await;
        match config.miniserver.get(&id) {
            Some(ms) => (ms.name.clone(), ms.ipaddress.clone()),
            None => {
                return Html(
                    "<div class='alert alert-danger'>Miniserver not found</div>".to_string(),
                )
            }
        }
    };

    let client = match state.get_miniserver_client(ms_id).await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to get Miniserver client for {}: {}", ms_name, e);
            return Html(format!(
                "<div class='alert alert-danger'>Cannot reach Miniserver '{}' ({}): {}</div>",
                ms_name, ms_ip, e
            ));
        }
    };

    info!("Starting backup of Miniserver '{}' ({})", ms_name, ms_ip);
    log_backup(
        &state.lbhomedir,
        "INFO",
        &format!("Manual backup started for '{}' ({})", ms_name, ms_ip),
    )
    .await;

    // Step 1: list all files from the Miniserver's /prog/ directory.
    // We search multiple paths and collect diagnostics for the error message.
    let (all_files, fsget_prefix) = {
        let search_paths: &[(&str, &str)] = &[
            ("/dev/fslist/prog/", "/dev/fsget/prog/"),
            ("/dev/fslist/", "/dev/fsget/"),
            ("/dev/fslist/sd/", "/dev/fsget/sd/"),
        ];
        let mut found: Option<(Vec<String>, String)> = None;
        let mut diagnostics: Vec<String> = Vec::new();

        for (fslist, fsget) in search_paths {
            match client.http().download_bytes(fslist).await {
                Ok((bytes, _)) => {
                    let listing = String::from_utf8_lossy(&bytes);
                    debug!("Listing from {} on '{}':\n{}", fslist, ms_name, listing);
                    let files = parse_all_files(&listing);
                    if !files.is_empty() {
                        info!(
                            "Found {} file(s) in {} on '{}'",
                            files.len(),
                            fslist,
                            ms_name
                        );
                        found = Some((files, fsget.to_string()));
                        break;
                    }
                    // Collect first 200 chars for diagnostics
                    let preview = listing.chars().take(200).collect::<String>();
                    diagnostics.push(format!(
                        "{}: OK but no files — preview: {:?}",
                        fslist, preview
                    ));
                }
                Err(e) => {
                    diagnostics.push(format!("{}: {}", fslist, e));
                }
            }
        }

        match found {
            Some(p) => p,
            None => {
                let diag_text = diagnostics.join("; ");
                log_backup(
                    &state.lbhomedir,
                    "ERROR",
                    &format!("No files found for '{}': {}", ms_name, diag_text),
                )
                .await;
                let diag = diagnostics.join("<br>");
                return Html(format!(
                    "<div class='alert alert-danger'>\
                     <strong>No files found on '{}'.</strong><br>\
                     <small>Paths checked:<br>{}</small>\
                     </div>",
                    ms_name, diag
                ));
            }
        }
    };

    info!("Found {} file(s) on '{}'", all_files.len(), ms_name);

    // Step 2: download every file and pack into a ZIP archive in memory
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let safe_name: String = ms_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let filename = format!("Backup_{}_{}.zip", safe_name, timestamp);

    let mut cursor = Cursor::new(Vec::<u8>::new());
    let mut downloaded = 0usize;
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for file in &all_files {
            let download_path = format!("{}{}", fsget_prefix, file);
            match client.http().download_bytes(&download_path).await {
                Ok((bytes, _)) => {
                    if zip.start_file(file, options).is_ok() {
                        if let Err(e) = zip.write_all(&bytes) {
                            warn!("Failed to write '{}' into ZIP: {}", file, e);
                        } else {
                            info!("  Packed '{}' ({} bytes)", file, bytes.len());
                            downloaded += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!("Skipping '{}' — download failed: {}", file, e);
                }
            }
        }

        if let Err(e) = zip.finish() {
            error!("Failed to finalise ZIP for '{}': {}", ms_name, e);
            log_backup(
                &state.lbhomedir,
                "ERROR",
                &format!("ZIP finalise failed for '{}': {}", ms_name, e),
            )
            .await;
            return Html(format!(
                "<div class='alert alert-danger'>Failed to create ZIP archive for '{}': {}</div>",
                ms_name, e
            ));
        }
    }
    let zip_bytes = cursor.into_inner();

    if downloaded == 0 {
        log_backup(
            &state.lbhomedir,
            "ERROR",
            &format!("No files could be downloaded from '{}'", ms_name),
        )
        .await;
        return Html(format!(
            "<div class='alert alert-danger'>No files could be downloaded from '{}'.</div>",
            ms_name
        ));
    }

    // Ensure target directory exists
    let base_dir = state.lbhomedir.join("data/system/miniserver-backups");
    let dir = ms_backup_dir(&base_dir, &id, &ms_name);
    if let Err(e) = fs::create_dir_all(&dir).await {
        error!("Failed to create backup dir {:?}: {}", dir, e);
        return Html(format!(
            "<div class='alert alert-danger'>Failed to create backup directory: {}</div>",
            e
        ));
    }

    // Write ZIP file
    let backup_path = dir.join(&filename);
    if let Err(e) = fs::write(&backup_path, &zip_bytes).await {
        error!("Failed to write backup file {:?}: {}", backup_path, e);
        return Html(format!(
            "<div class='alert alert-danger'>Failed to save backup: {}</div>",
            e
        ));
    }

    info!(
        "Backup saved: {:?} ({} files, {} bytes)",
        backup_path,
        downloaded,
        zip_bytes.len()
    );
    log_backup(
        &state.lbhomedir,
        "INFO",
        &format!(
            "Manual backup completed for '{}': {} files → {} ({})",
            ms_name,
            downloaded,
            filename,
            format_size(zip_bytes.len() as u64)
        ),
    )
    .await;

    // Rotate old backups
    rotate_backups(&dir).await;

    Html(format!(
        "<div class='alert alert-success'>Backup of <strong>{}</strong> completed: \
         {} file(s) → {} ({}) \
         &mdash; <a href='/miniserver/backup'>Refresh</a> to see the updated list.</div>",
        ms_name,
        downloaded,
        filename,
        format_size(zip_bytes.len() as u64),
    ))
}

/// Download a backup file to the browser.
pub async fn download(
    State(state): State<AppState>,
    Path((id, filename)): Path<(String, String)>,
) -> Response {
    // Reject path traversal attempts
    if filename.contains('/') || filename.contains("..") {
        return (StatusCode::BAD_REQUEST, "Invalid filename").into_response();
    }

    let ms_name = {
        let config = state.config.read().await;
        match config.miniserver.get(&id) {
            Some(ms) => ms.name.clone(),
            None => return (StatusCode::NOT_FOUND, "Miniserver not found").into_response(),
        }
    };

    let base_dir = state.lbhomedir.join("data/system/miniserver-backups");
    let path = ms_backup_dir(&base_dir, &id, &ms_name).join(&filename);

    match fs::read(&path).await {
        Ok(bytes) => {
            let cd = format!("attachment; filename=\"{}\"", filename);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .header(header::CONTENT_DISPOSITION, cd)
                .body(Body::from(bytes))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        Err(_) => (StatusCode::NOT_FOUND, "Backup file not found").into_response(),
    }
}

/// Delete a single backup file (HTMX endpoint — returns empty to remove the row).
pub async fn delete_backup(
    State(state): State<AppState>,
    Path((id, filename)): Path<(String, String)>,
) -> Html<String> {
    if filename.contains('/') || filename.contains("..") {
        return Html(
            "<tr><td colspan='4' class='alert alert-danger'>Invalid filename</td></tr>".to_string(),
        );
    }

    let ms_name =
        {
            let config = state.config.read().await;
            match config.miniserver.get(&id) {
                Some(ms) => ms.name.clone(),
                None => return Html(
                    "<tr><td colspan='4' class='alert alert-danger'>Miniserver not found</td></tr>"
                        .to_string(),
                ),
            }
        };

    let base_dir = state.lbhomedir.join("data/system/miniserver-backups");
    let path = ms_backup_dir(&base_dir, &id, &ms_name).join(&filename);

    match fs::remove_file(&path).await {
        Ok(()) => Html(String::new()), // empty = hx-swap outerHTML removes the row
        Err(e) => Html(format!(
            "<tr><td colspan='4' class='alert alert-danger'>Delete failed: {}</td></tr>",
            e
        )),
    }
}

// ---------------------------------------------------------------------------
// Schedule settings
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ScheduleFormData {
    /// Checkbox — absent when unchecked, so Option
    pub enabled: Option<String>,
    pub interval_hours: u32,
}

/// Save the backup schedule for a Miniserver (HTMX endpoint).
pub async fn save_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Form(form): Form<ScheduleFormData>,
) -> Html<String> {
    let mut schedules = load_ms_schedules(&state.lbhomedir).await;
    let entry = schedules
        .schedules
        .entry(id.clone())
        .or_insert_with(MsBackupSchedule::default);
    entry.enabled = form.enabled.is_some();
    entry.interval_hours = form.interval_hours.max(1);

    match save_ms_schedules(&state.lbhomedir, &schedules).await {
        Ok(()) => {
            let enabled = form.enabled.is_some();
            let interval = form.interval_hours.max(1);
            log_backup(
                &state.lbhomedir,
                "INFO",
                &format!(
                    "Schedule updated for Miniserver {}: enabled={}, interval={}h",
                    id, enabled, interval
                ),
            )
            .await;
            Html("<div class='alert alert-success'>Schedule saved.</div>".to_string())
        }
        Err(e) => Html(format!(
            "<div class='alert alert-danger'>Failed to save schedule: {}</div>",
            e
        )),
    }
}

// ---------------------------------------------------------------------------
// Background scheduler
// ---------------------------------------------------------------------------

/// Perform a backup for a single Miniserver by ID. Returns Ok(filename) or Err(message).
async fn do_backup_for(state: &AppState, id: &str) -> Result<String, String> {
    let ms_id: u8 = id
        .parse()
        .map_err(|_| "Invalid Miniserver ID".to_string())?;

    let (ms_name, ms_ip) = {
        let config = state.config.read().await;
        match config.miniserver.get(id) {
            Some(ms) => (ms.name.clone(), ms.ipaddress.clone()),
            None => return Err(format!("Miniserver {} not found in config", id)),
        }
    };

    let client = state
        .get_miniserver_client(ms_id)
        .await
        .map_err(|e| format!("Cannot reach Miniserver '{}' ({}): {}", ms_name, ms_ip, e))?;

    info!(
        "Scheduled backup starting for Miniserver '{}' ({})",
        ms_name, ms_ip
    );
    log_backup(
        &state.lbhomedir,
        "INFO",
        &format!("Scheduled backup started for '{}' ({})", ms_name, ms_ip),
    )
    .await;

    let search_paths: &[(&str, &str)] = &[
        ("/dev/fslist/prog/", "/dev/fsget/prog/"),
        ("/dev/fslist/", "/dev/fsget/"),
    ];
    let mut found: Option<(Vec<String>, String)> = None;
    for (fslist, fsget) in search_paths {
        if let Ok((bytes, _)) = client.http().download_bytes(fslist).await {
            let listing = String::from_utf8_lossy(&bytes);
            let files = parse_all_files(&listing);
            if !files.is_empty() {
                found = Some((files, fsget.to_string()));
                break;
            }
        }
    }

    let (all_files, fsget_prefix) =
        found.ok_or_else(|| format!("No files found on '{}'", ms_name))?;

    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let safe_name: String = ms_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let filename = format!("Backup_{}_{}.zip", safe_name, timestamp);

    // Download all files and pack into a ZIP archive
    let mut cursor = Cursor::new(Vec::<u8>::new());
    let mut downloaded = 0usize;
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for file in &all_files {
            let download_path = format!("{}{}", fsget_prefix, file);
            match client.http().download_bytes(&download_path).await {
                Ok((bytes, _)) => {
                    if zip.start_file(file, options).is_ok() {
                        if zip.write_all(&bytes).is_ok() {
                            downloaded += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!("Skipping '{}' in scheduled backup: {}", file, e);
                }
            }
        }

        zip.finish()
            .map_err(|e| format!("Failed to finalise ZIP for '{}': {}", ms_name, e))?;
    }
    let zip_bytes = cursor.into_inner();

    if downloaded == 0 {
        return Err(format!("No files could be downloaded from '{}'", ms_name));
    }

    let base_dir = state.lbhomedir.join("data/system/miniserver-backups");
    let dir = ms_backup_dir(&base_dir, id, &ms_name);
    fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("Failed to create backup dir: {}", e))?;

    let backup_path = dir.join(&filename);
    fs::write(&backup_path, &zip_bytes)
        .await
        .map_err(|e| format!("Failed to write backup: {}", e))?;

    info!(
        "Scheduled backup saved: {:?} ({} files, {} bytes)",
        backup_path,
        downloaded,
        zip_bytes.len()
    );
    log_backup(
        &state.lbhomedir,
        "INFO",
        &format!(
            "Scheduled backup completed for '{}': {} files → {} ({})",
            ms_name,
            downloaded,
            filename,
            format_size(zip_bytes.len() as u64)
        ),
    )
    .await;
    rotate_backups(&dir).await;

    Ok(filename)
}

/// Spawn a background task that runs automatic MS backups according to saved schedules.
pub fn spawn_ms_backup_scheduler(state: AppState) {
    tokio::spawn(async move {
        // Check every 30 minutes whether any backup is due
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(30 * 60));
        ticker.tick().await; // skip the immediate first tick

        loop {
            ticker.tick().await;

            let mut schedules = load_ms_schedules(&state.lbhomedir).await;
            let now_ts = Utc::now().timestamp();
            let mut changed = false;

            let ids: Vec<String> = schedules.schedules.keys().cloned().collect();
            for id in ids {
                let sched = match schedules.schedules.get(&id) {
                    Some(s) if s.enabled => s.clone(),
                    _ => continue,
                };

                let interval_secs = (sched.interval_hours as i64) * 3600;
                let last = sched.last_run_ts.unwrap_or(0);
                if now_ts - last < interval_secs {
                    continue; // not due yet
                }

                match do_backup_for(&state, &id).await {
                    Ok(fname) => {
                        info!("Scheduled MS backup OK for {}: {}", id, fname);
                        if let Some(s) = schedules.schedules.get_mut(&id) {
                            s.last_run_ts = Some(now_ts);
                            changed = true;
                        }
                    }
                    Err(e) => {
                        error!("Scheduled MS backup failed for {}: {}", id, e);
                        log_backup(
                            &state.lbhomedir,
                            "ERROR",
                            &format!("Scheduled backup failed for {}: {}", id, e),
                        )
                        .await;
                    }
                }
            }

            if changed {
                if let Err(e) = save_ms_schedules(&state.lbhomedir, &schedules).await {
                    warn!("Failed to save MS backup schedule after run: {}", e);
                }
            }
        }
    });
}
