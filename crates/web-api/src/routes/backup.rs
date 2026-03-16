//! Backup and restore API routes

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use backup_manager::{BackupManager, BackupMetadata};
use serde::{Deserialize, Serialize};
use tokio_util::io::ReaderStream;

use crate::AppState;

#[derive(Debug, Serialize)]
pub struct BackupResponse {
    pub name: String,
    pub size_bytes: u64,
    pub created: String,
    pub metadata: Option<BackupMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBackupQuery {
    #[serde(default = "default_true")]
    pub include_plugins: bool,
}

fn default_true() -> bool {
    true
}

/// List all backups
pub async fn list_backups(State(state): State<AppState>) -> Json<Vec<BackupResponse>> {
    let manager = BackupManager::new(state.lbhomedir.clone());

    let backups = match manager.list_backups().await {
        Ok(b) => b
            .into_iter()
            .map(|b| BackupResponse {
                name: b.name,
                size_bytes: b.size_bytes,
                created: b.created.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                metadata: b.metadata,
            })
            .collect(),
        Err(e) => {
            tracing::error!("Failed to list backups: {}", e);
            Vec::new()
        }
    };

    Json(backups)
}

/// Create a new backup
pub async fn create_backup(
    State(state): State<AppState>,
    Query(query): Query<CreateBackupQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let manager = BackupManager::new(state.lbhomedir.clone());

    match manager.create_backup(query.include_plugins).await {
        Ok(path) => {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            Ok(Json(serde_json::json!({
                "success": true,
                "backup_name": name,
                "message": "Backup created successfully"
            })))
        }
        Err(e) => {
            tracing::error!("Failed to create backup: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": e.to_string()
                })),
            ))
        }
    }
}

/// Download a backup file
pub async fn download_backup(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Prevent path traversal
    if name.contains('/') || name.contains("..") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid backup name"})),
        ));
    }

    let backup_dir = backup_manager::backup_dir(&state.lbhomedir);
    let path = backup_dir.join(&name);

    if !path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Backup not found"})),
        ));
    }

    let file = tokio::fs::File::open(&path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/gzip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", name),
        )
        .body(body)
        .unwrap())
}

/// Restore from a backup
pub async fn restore_backup(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Prevent path traversal
    if name.contains('/') || name.contains("..") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid backup name"})),
        ));
    }

    let backup_dir = backup_manager::backup_dir(&state.lbhomedir);
    let backup_path = backup_dir.join(&name);

    if !backup_path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Backup not found"})),
        ));
    }

    match backup_manager::restore_backup(state.lbhomedir.clone(), backup_path).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Backup restored successfully. Restart may be required to apply all changes."
        }))),
        Err(e) => {
            tracing::error!("Failed to restore backup {}: {}", name, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": e.to_string()
                })),
            ))
        }
    }
}

/// Delete a backup
pub async fn delete_backup(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Prevent path traversal
    if name.contains('/') || name.contains("..") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid backup name"})),
        ));
    }

    let manager = BackupManager::new(state.lbhomedir.clone());

    match manager.delete_backup(&name).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Backup deleted successfully"
        }))),
        Err(e) => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}
