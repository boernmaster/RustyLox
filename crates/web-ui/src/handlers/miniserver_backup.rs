//! Miniserver backup handlers
//!
//! Performs a full backup of the Miniserver filesystem via its HTTP API:
//!   GET /dev/fslist/<dir>/  — list a directory (recursive BFS walk)
//!   GET /dev/fsget/<path>   — download a file
//!
//! All standard directories (log, prog, sys, stats, temp, update, web, user)
//! are walked recursively and packed into a timestamped ZIP archive.
//! Backups are stored under `$LBHOMEDIR/data/system/miniserver-backups/<id>-<name>/`.

use askama::Template;
use axum::response::sse::{Event, KeepAlive};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response, Sse},
    Form,
};
use chrono::{Local, TimeZone, Utc};
use futures::stream::Stream;
use miniserver_client::MiniserverClient;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::io::{Cursor, Write};
use std::path::{Path as StdPath, PathBuf};
use std::sync::LazyLock;
use tokio::fs;
use tracing::{debug, error, info, warn};
use web_api::AppState;

// ---------------------------------------------------------------------------
// SSE progress tracking for in-flight backup jobs
// ---------------------------------------------------------------------------

/// Each progress event is (event_name, json_data).
type ProgressPayload = (String, String);

static BACKUP_JOBS: LazyLock<
    std::sync::Mutex<HashMap<String, tokio::sync::mpsc::UnboundedReceiver<ProgressPayload>>>,
> = LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

fn new_job_id() -> String {
    format!("{}", Local::now().timestamp_micros())
}

const BACKUPS_TO_KEEP: usize = 7;

/// Miniserver filesystem directories included in a full backup.
const BACKUP_DIRS: &[&str] = &[
    "log", "prog", "sys", "stats", "temp", "update", "web", "user",
];

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

async fn log_backup(lbhomedir: &StdPath, level: &str, message: &str) {
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

async fn load_ms_schedules(lbhomedir: &StdPath) -> MsBackupSchedules {
    let path = lbhomedir.join("config/system/ms_backup_schedule.json");
    let Ok(content) = fs::read_to_string(&path).await else {
        return MsBackupSchedules::default();
    };
    serde_json::from_str(&content).unwrap_or_default()
}

async fn save_ms_schedules(
    lbhomedir: &StdPath,
    schedules: &MsBackupSchedules,
) -> std::io::Result<()> {
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
    pub lang: String,
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
fn ms_backup_dir(base: &StdPath, id: &str, name: &str) -> PathBuf {
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
async fn list_backups_for(dir: &StdPath) -> Vec<BackupFileDisplay> {
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
async fn rotate_backups(dir: &StdPath) {
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

    let (ms_entries, lang) = {
        let config = state.config.read().await;
        let entries: Vec<(String, String, String)> = config
            .miniserver
            .iter()
            .map(|(id, ms)| (id.clone(), ms.name.clone(), ms.ipaddress.clone()))
            .collect();
        (entries, config.base.lang.clone())
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
        lang,
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// Walk a Miniserver filesystem directory recursively (BFS).
///
/// Returns full paths like `/prog/Default.Loxone`, `/log/2024-01-01.log`, etc.
/// Sub-directories are discovered via `d` lines and queued automatically.
/// `/sys/internal/` is skipped — it errors on many Miniserver firmware versions.
async fn walk_ms_dir(client: &MiniserverClient, root: &str) -> Vec<String> {
    let mut all_files = Vec::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(root.to_string());

    while let Some(dir) = queue.pop_front() {
        if dir == "/sys/internal/" {
            continue; // known to error on many firmware versions
        }
        let url = format!("/dev/fslist{}", dir);
        let listing = match client.http().download_bytes(&url).await {
            Ok((bytes, _)) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) => {
                debug!("Cannot list {}: {}", url, e);
                continue;
            }
        };
        for line in listing.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(name) = line.split_whitespace().last() {
                if line.starts_with("d ") {
                    queue.push_back(format!("{}{}/", dir, name));
                } else {
                    all_files.push(format!("{}{}", dir, name));
                }
            }
        }
    }
    all_files
}

/// Trigger a backup for one Miniserver (HTMX endpoint).
///
/// Quickly validates the Miniserver is reachable, then spawns a background
/// task and returns a job-trigger span. The browser connects to the SSE
/// endpoint to receive real-time progress events.
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

    // Fast connectivity check before spawning
    if let Err(e) = state.get_miniserver_client(ms_id).await {
        error!("Failed to get Miniserver client for {}: {}", ms_name, e);
        return Html(format!(
            "<div class='alert alert-danger'>Cannot reach Miniserver '{}' ({}): {}</div>",
            ms_name, ms_ip, e
        ));
    }

    let job_id = new_job_id();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<ProgressPayload>();
    {
        let mut jobs = BACKUP_JOBS.lock().unwrap();
        jobs.insert(job_id.clone(), rx);
    }

    info!(
        "Spawning backup task for '{}' ({}) — job {}",
        ms_name, ms_ip, job_id
    );
    log_backup(
        &state.lbhomedir,
        "INFO",
        &format!("Manual backup started for '{}' ({})", ms_name, ms_ip),
    )
    .await;

    tokio::spawn(run_backup_task(state, id.clone(), job_id.clone(), tx));

    // Return a sentinel span; JS will pick up the data attributes and open SSE.
    Html(format!(
        r#"<span data-backup-job="{}" data-ms-id="{}"></span>"#,
        job_id, id
    ))
}

/// Background task: walk all Miniserver directories, download files, build ZIP,
/// emit SSE progress events throughout.
async fn run_backup_task(
    state: AppState,
    id: String,
    job_id: String,
    tx: tokio::sync::mpsc::UnboundedSender<ProgressPayload>,
) {
    // Helper: send an SSE event (best-effort; receiver may have disconnected)
    let send = |name: &str, data: String| {
        let _ = tx.send((name.to_string(), data));
    };

    let ms_id: u8 = match id.parse() {
        Ok(n) => n,
        Err(_) => {
            send(
                "backup_error",
                r#"{"message":"Invalid Miniserver ID"}"#.to_string(),
            );
            return;
        }
    };

    let (ms_name, ms_ip) = {
        let config = state.config.read().await;
        match config.miniserver.get(&id) {
            Some(ms) => (ms.name.clone(), ms.ipaddress.clone()),
            None => {
                send(
                    "backup_error",
                    r#"{"message":"Miniserver not found in config"}"#.to_string(),
                );
                return;
            }
        }
    };

    let client = match state.get_miniserver_client(ms_id).await {
        Ok(c) => c,
        Err(e) => {
            let msg = serde_json::json!({
                "message": format!("Cannot reach Miniserver '{}' ({}): {}", ms_name, ms_ip, e)
            })
            .to_string();
            send("backup_error", msg);
            return;
        }
    };

    // --- Walk all directories to build the file list ---
    let mut all_files: Vec<String> = Vec::new();
    for dir in BACKUP_DIRS {
        let dir_path = format!("/{}/", dir);
        let files = walk_ms_dir(&client, &dir_path).await;
        info!(
            "Listed {} file(s) in {} on '{}'",
            files.len(),
            dir_path,
            ms_name
        );
        all_files.extend(files);
    }

    if all_files.is_empty() {
        log_backup(
            &state.lbhomedir,
            "ERROR",
            &format!("No files found on '{}'", ms_name),
        )
        .await;
        send(
            "backup_error",
            serde_json::json!({"message": format!("No files found on '{}'", ms_name)}).to_string(),
        );
        return;
    }

    let total = all_files.len();
    info!("Total: {} file(s) to back up from '{}'", total, ms_name);
    send("start", serde_json::json!({"total": total}).to_string());

    // --- Download every file and pack into a ZIP ---
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

        for (i, file) in all_files.iter().enumerate() {
            let download_url = format!("/dev/fsget{}", file);
            let zip_entry = file.trim_start_matches('/');
            match client.http().download_bytes(&download_url).await {
                Ok((bytes, _)) => {
                    if zip.start_file(zip_entry, options).is_ok() {
                        if let Err(e) = zip.write_all(&bytes) {
                            warn!("Failed to write '{}' into ZIP: {}", zip_entry, e);
                        } else {
                            debug!("  Packed '{}' ({} bytes)", zip_entry, bytes.len());
                            downloaded += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!("Skipping '{}' — download failed: {}", file, e);
                }
            }
            // Emit progress after each file attempt (success or skip)
            send(
                "file",
                serde_json::json!({
                    "done": i + 1,
                    "total": total,
                    "file": zip_entry,
                })
                .to_string(),
            );
        }

        if let Err(e) = zip.finish() {
            error!("Failed to finalise ZIP for '{}': {}", ms_name, e);
            log_backup(
                &state.lbhomedir,
                "ERROR",
                &format!("ZIP finalise failed for '{}': {}", ms_name, e),
            )
            .await;
            send(
                "backup_error",
                serde_json::json!({"message": format!("Failed to finalise ZIP: {}", e)})
                    .to_string(),
            );
            return;
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
        send(
            "backup_error",
            serde_json::json!({"message": format!("No files could be downloaded from '{}'", ms_name)})
                .to_string(),
        );
        return;
    }

    // --- Save to disk ---
    let base_dir = state.lbhomedir.join("data/system/miniserver-backups");
    let dir = ms_backup_dir(&base_dir, &id, &ms_name);
    if let Err(e) = fs::create_dir_all(&dir).await {
        error!("Failed to create backup dir {:?}: {}", dir, e);
        send(
            "backup_error",
            serde_json::json!({"message": format!("Failed to create backup directory: {}", e)})
                .to_string(),
        );
        return;
    }

    let backup_path = dir.join(&filename);
    if let Err(e) = fs::write(&backup_path, &zip_bytes).await {
        error!("Failed to write backup file {:?}: {}", backup_path, e);
        send(
            "backup_error",
            serde_json::json!({"message": format!("Failed to save backup file: {}", e)})
                .to_string(),
        );
        return;
    }

    let size_str = format_size(zip_bytes.len() as u64);
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
            ms_name, downloaded, filename, size_str
        ),
    )
    .await;

    rotate_backups(&dir).await;

    send(
        "done",
        serde_json::json!({
            "filename": filename,
            "size": size_str,
            "count": downloaded,
        })
        .to_string(),
    );

    // Clean up job slot (normally the SSE handler already consumed it)
    let mut jobs = BACKUP_JOBS.lock().unwrap();
    jobs.remove(&job_id);
}

/// Stream real-time backup progress as Server-Sent Events.
pub async fn backup_progress(
    Path((_id, job_id)): Path<(String, String)>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = {
        let mut jobs = BACKUP_JOBS.lock().unwrap();
        jobs.remove(&job_id)
    };

    let stream = async_stream::stream! {
        let Some(mut rx) = rx else {
            yield Ok(Event::default()
                .event("backup_error")
                .data(r#"{"message":"Backup job not found or already consumed"}"#));
            return;
        };
        while let Some((event_name, data)) = rx.recv().await {
            yield Ok(Event::default().event(event_name).data(data));
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
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

    // Walk all backup directories recursively
    let mut all_files: Vec<String> = Vec::new();
    for dir in BACKUP_DIRS {
        let dir_path = format!("/{}/", dir);
        let files = walk_ms_dir(&client, &dir_path).await;
        info!(
            "Listed {} file(s) in {} on '{}'",
            files.len(),
            dir_path,
            ms_name
        );
        all_files.extend(files);
    }

    if all_files.is_empty() {
        return Err(format!("No files found on '{}'", ms_name));
    }

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
            let download_url = format!("/dev/fsget{}", file);
            let zip_entry = file.trim_start_matches('/');
            match client.http().download_bytes(&download_url).await {
                Ok((bytes, _)) => {
                    if zip.start_file(zip_entry, options).is_ok() && zip.write_all(&bytes).is_ok() {
                        downloaded += 1;
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
