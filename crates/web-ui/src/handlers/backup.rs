//! Backup and restore UI handlers

use askama::Template;
use axum::extract::State;
use axum::{
    extract::{Path, Query},
    response::Html,
};
use backup_manager::BackupManager;
use serde::Deserialize;
use web_api::AppState;

#[derive(Debug)]
pub struct BackupDisplay {
    pub name: String,
    pub size_human: String,
    pub created: String,
}

#[derive(Template)]
#[template(path = "backup.html")]
pub struct BackupTemplate {
    pub backups: Vec<BackupDisplay>,
    pub version: String,
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Show backup management page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let manager = BackupManager::new(state.lbhomedir.clone());

    let backups = match manager.list_backups().await {
        Ok(list) => list
            .into_iter()
            .map(|b| BackupDisplay {
                name: b.name,
                size_human: format_size(b.size_bytes),
                created: b.created.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            })
            .collect(),
        Err(e) => {
            tracing::error!("Failed to list backups: {}", e);
            Vec::new()
        }
    };

    let template = BackupTemplate {
        backups,
        version: state.version.clone(),
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

#[derive(Debug, Deserialize)]
pub struct CreateBackupQuery {
    #[serde(default = "default_true")]
    pub include_plugins: bool,
}

fn default_true() -> bool {
    true
}

/// Create a backup (HTMX endpoint)
pub async fn create(
    State(state): State<AppState>,
    Query(query): Query<CreateBackupQuery>,
) -> Html<String> {
    let manager = BackupManager::new(state.lbhomedir.clone());

    match manager.create_backup(query.include_plugins).await {
        Ok(path) => {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            Html(format!(
                "<div class='success'>Backup <strong>{}</strong> created successfully. \
                 <a href='/backup'>Refresh</a> to see it in the list.</div>",
                name
            ))
        }
        Err(e) => Html(format!(
            "<div class='error'>Failed to create backup: {}</div>",
            e
        )),
    }
}

/// Restore from a backup (HTMX endpoint)
pub async fn restore(State(state): State<AppState>, Path(name): Path<String>) -> Html<String> {
    // Prevent path traversal
    if name.contains('/') || name.contains("..") {
        return Html("<div class='error'>Invalid backup name</div>".to_string());
    }

    let backup_dir = backup_manager::backup_dir(&state.lbhomedir);
    let backup_path = backup_dir.join(&name);

    match backup_manager::restore_backup(state.lbhomedir.clone(), backup_path).await {
        Ok(()) => Html(format!(
            "<div class='success'>Restored from <strong>{}</strong> successfully. \
             Configuration has been updated. A restart may be required to apply all changes.</div>",
            name
        )),
        Err(e) => Html(format!(
            "<div class='error'>Failed to restore backup: {}</div>",
            e
        )),
    }
}

/// Delete a backup (HTMX endpoint - removes the row)
pub async fn delete(State(state): State<AppState>, Path(name): Path<String>) -> Html<String> {
    // Prevent path traversal
    if name.contains('/') || name.contains("..") {
        return Html("<div class='error'>Invalid backup name</div>".to_string());
    }

    let manager = BackupManager::new(state.lbhomedir.clone());

    match manager.delete_backup(&name).await {
        Ok(()) => Html(String::new()), // Empty response removes the row via hx-swap outerHTML
        Err(e) => Html(format!("<tr><td colspan='4' class='error'>{}</td></tr>", e)),
    }
}
