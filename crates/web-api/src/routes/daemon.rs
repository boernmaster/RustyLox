//! Plugin daemon management API endpoints

use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use plugin_manager::{DaemonInfo, DaemonManager, PluginInstaller};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

/// Daemon log query parameters
#[derive(Debug, Deserialize)]
pub struct LogParams {
    /// Number of lines to return (default 100)
    pub lines: Option<usize>,
}

/// Daemon action response
#[derive(Debug, Serialize)]
pub struct DaemonActionResponse {
    pub success: bool,
    pub daemon: Option<DaemonInfo>,
    pub error: Option<String>,
}

/// Start plugin daemon
///
/// POST /api/plugins/:folder/daemon/start
pub async fn start_daemon(
    State(state): State<AppState>,
    Path(folder): Path<String>,
) -> impl IntoResponse {
    info!("Starting daemon for plugin: {}", folder);

    let lbhomedir = &state.lbhomedir;
    let installer = PluginInstaller::new(lbhomedir);
    let daemon_manager = DaemonManager::new(lbhomedir);

    // Find plugin by folder name
    let plugin = match find_plugin_by_folder(&installer, &folder).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(DaemonActionResponse {
                    success: false,
                    daemon: None,
                    error: Some(format!("Plugin '{}' not found", folder)),
                }),
            )
                .into_response();
        }
        Err(e) => {
            error!("Failed to find plugin: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DaemonActionResponse {
                    success: false,
                    daemon: None,
                    error: Some(format!("Failed to find plugin: {}", e)),
                }),
            )
                .into_response();
        }
    };

    match daemon_manager.start(&plugin).await {
        Ok(info) => (
            StatusCode::OK,
            Json(DaemonActionResponse {
                success: true,
                daemon: Some(info),
                error: None,
            }),
        )
            .into_response(),
        Err(e) => {
            error!("Failed to start daemon for {}: {}", folder, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DaemonActionResponse {
                    success: false,
                    daemon: None,
                    error: Some(format!("{}", e)),
                }),
            )
                .into_response()
        }
    }
}

/// Stop plugin daemon
///
/// POST /api/plugins/:folder/daemon/stop
pub async fn stop_daemon(
    State(state): State<AppState>,
    Path(folder): Path<String>,
) -> impl IntoResponse {
    info!("Stopping daemon for plugin: {}", folder);

    let lbhomedir = &state.lbhomedir;
    let installer = PluginInstaller::new(lbhomedir);
    let daemon_manager = DaemonManager::new(lbhomedir);

    let plugin = match find_plugin_by_folder(&installer, &folder).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(DaemonActionResponse {
                    success: false,
                    daemon: None,
                    error: Some(format!("Plugin '{}' not found", folder)),
                }),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DaemonActionResponse {
                    success: false,
                    daemon: None,
                    error: Some(format!("{}", e)),
                }),
            )
                .into_response();
        }
    };

    match daemon_manager.stop(&plugin).await {
        Ok(info) => (
            StatusCode::OK,
            Json(DaemonActionResponse {
                success: true,
                daemon: Some(info),
                error: None,
            }),
        )
            .into_response(),
        Err(e) => {
            error!("Failed to stop daemon for {}: {}", folder, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DaemonActionResponse {
                    success: false,
                    daemon: None,
                    error: Some(format!("{}", e)),
                }),
            )
                .into_response()
        }
    }
}

/// Restart plugin daemon
///
/// POST /api/plugins/:folder/daemon/restart
pub async fn restart_daemon(
    State(state): State<AppState>,
    Path(folder): Path<String>,
) -> impl IntoResponse {
    info!("Restarting daemon for plugin: {}", folder);

    let lbhomedir = &state.lbhomedir;
    let installer = PluginInstaller::new(lbhomedir);
    let daemon_manager = DaemonManager::new(lbhomedir);

    let plugin = match find_plugin_by_folder(&installer, &folder).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(DaemonActionResponse {
                    success: false,
                    daemon: None,
                    error: Some(format!("Plugin '{}' not found", folder)),
                }),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DaemonActionResponse {
                    success: false,
                    daemon: None,
                    error: Some(format!("{}", e)),
                }),
            )
                .into_response();
        }
    };

    match daemon_manager.restart(&plugin).await {
        Ok(info) => (
            StatusCode::OK,
            Json(DaemonActionResponse {
                success: true,
                daemon: Some(info),
                error: None,
            }),
        )
            .into_response(),
        Err(e) => {
            error!("Failed to restart daemon for {}: {}", folder, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DaemonActionResponse {
                    success: false,
                    daemon: None,
                    error: Some(format!("{}", e)),
                }),
            )
                .into_response()
        }
    }
}

/// Get daemon status
///
/// GET /api/plugins/:folder/daemon/status
pub async fn get_daemon_status(
    State(state): State<AppState>,
    Path(folder): Path<String>,
) -> impl IntoResponse {
    let lbhomedir = &state.lbhomedir;
    let installer = PluginInstaller::new(lbhomedir);
    let daemon_manager = DaemonManager::new(lbhomedir);

    let plugin = match find_plugin_by_folder(&installer, &folder).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": format!("Plugin '{}' not found", folder) })),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("{}", e) })),
            )
                .into_response();
        }
    };

    let info = daemon_manager.status(&plugin).await;
    Json(info).into_response()
}

/// Get daemon logs
///
/// GET /api/plugins/:folder/daemon/logs
pub async fn get_daemon_logs(
    State(state): State<AppState>,
    Path(folder): Path<String>,
    Query(params): Query<LogParams>,
) -> impl IntoResponse {
    let lbhomedir = &state.lbhomedir;
    let installer = PluginInstaller::new(lbhomedir);
    let daemon_manager = DaemonManager::new(lbhomedir);
    let lines = params.lines.unwrap_or(100);

    let plugin = match find_plugin_by_folder(&installer, &folder).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": format!("Plugin '{}' not found", folder) })),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("{}", e) })),
            )
                .into_response();
        }
    };

    match daemon_manager.get_logs(&plugin, lines).await {
        Ok(logs) => Json(serde_json::json!({
            "plugin": folder,
            "lines": lines,
            "logs": logs
        }))
        .into_response(),
        Err(e) => {
            error!("Failed to get daemon logs for {}: {}", folder, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("{}", e) })),
            )
                .into_response()
        }
    }
}

/// Helper: find plugin by folder name
async fn find_plugin_by_folder(
    installer: &PluginInstaller,
    folder: &str,
) -> loxberry_core::Result<Option<plugin_manager::PluginEntry>> {
    let plugins = installer.list().await?;
    Ok(plugins.into_iter().find(|p| p.folder == folder))
}
