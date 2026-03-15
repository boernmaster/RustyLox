//! Plugin management API endpoints

use crate::state::AppState;
use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use plugin_manager::{InstallAction, InstallRequest, PluginEntry, PluginInstaller};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;
use tracing::{error, info};

/// Plugin list response
#[derive(Debug, Serialize)]
pub struct PluginListResponse {
    pub plugins: Vec<PluginEntry>,
    pub count: usize,
}

/// Plugin install response
#[derive(Debug, Serialize)]
pub struct PluginInstallResponse {
    pub success: bool,
    pub plugin: Option<PluginEntry>,
    pub error: Option<String>,
}

/// Plugin uninstall response
#[derive(Debug, Serialize)]
pub struct PluginUninstallResponse {
    pub success: bool,
    pub message: String,
}

/// List all installed plugins
///
/// GET /api/plugins
pub async fn list_plugins(State(state): State<AppState>) -> impl IntoResponse {
    let lbhomedir = &state.lbhomedir;

    let installer = PluginInstaller::new(lbhomedir);

    match installer.list().await {
        Ok(plugins) => {
            let count = plugins.len();
            Json(PluginListResponse { plugins, count }).into_response()
        }
        Err(e) => {
            error!("Failed to list plugins: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to list plugins: {}", e)
                })),
            )
                .into_response()
        }
    }
}

/// Get plugin details by MD5
///
/// GET /api/plugins/:md5
pub async fn get_plugin(
    State(state): State<AppState>,
    Path(md5): Path<String>,
) -> impl IntoResponse {
    let lbhomedir = &state.lbhomedir;

    let installer = PluginInstaller::new(lbhomedir);

    match installer.get(&md5).await {
        Ok(Some(plugin)) => Json(plugin).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Plugin not found"
            })),
        )
            .into_response(),
        Err(e) => {
            error!("Failed to get plugin: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to get plugin: {}", e)
                })),
            )
                .into_response()
        }
    }
}

/// Install plugin from uploaded ZIP file
///
/// POST /api/plugins/install
pub async fn install_plugin(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    info!("Received plugin install request");

    // Extract ZIP file from multipart form data
    let mut temp_file: Option<NamedTempFile> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let data = match field.bytes().await {
                Ok(data) => data,
                Err(e) => {
                    error!("Failed to read upload: {}", e);
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(PluginInstallResponse {
                            success: false,
                            plugin: None,
                            error: Some(format!("Failed to read upload: {}", e)),
                        }),
                    )
                        .into_response();
                }
            };

            // Create temp file
            let mut tf = match NamedTempFile::new() {
                Ok(tf) => tf,
                Err(e) => {
                    error!("Failed to create temp file: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(PluginInstallResponse {
                            success: false,
                            plugin: None,
                            error: Some(format!("Failed to create temp file: {}", e)),
                        }),
                    )
                        .into_response();
                }
            };

            // Write data to temp file
            if let Err(e) = tokio::fs::File::from_std(tf.reopen().unwrap())
                .write_all(&data)
                .await
            {
                error!("Failed to write temp file: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(PluginInstallResponse {
                        success: false,
                        plugin: None,
                        error: Some(format!("Failed to write temp file: {}", e)),
                    }),
                )
                    .into_response();
            }

            temp_file = Some(tf);
            break;
        }
    }

    let temp_file = match temp_file {
        Some(tf) => tf,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(PluginInstallResponse {
                    success: false,
                    plugin: None,
                    error: Some("No file uploaded".to_string()),
                }),
            )
                .into_response();
        }
    };

    // Install plugin
    let lbhomedir = &state.lbhomedir;

    let installer = PluginInstaller::new(lbhomedir);

    let request = InstallRequest {
        zip_path: temp_file.path().to_path_buf(),
        action: InstallAction::Install,
        force: false,
    };

    match installer.install(request).await {
        Ok(plugin) => {
            info!("Successfully installed plugin: {} v{}", plugin.name, plugin.version);
            (
                StatusCode::OK,
                Json(PluginInstallResponse {
                    success: true,
                    plugin: Some(plugin),
                    error: None,
                }),
            )
                .into_response()
        }
        Err(e) => {
            error!("Failed to install plugin: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PluginInstallResponse {
                    success: false,
                    plugin: None,
                    error: Some(format!("{}", e)),
                }),
            )
                .into_response()
        }
    }
}

/// Uninstall plugin by MD5
///
/// DELETE /api/plugins/:md5
pub async fn uninstall_plugin(
    State(state): State<AppState>,
    Path(md5): Path<String>,
) -> impl IntoResponse {
    info!("Uninstalling plugin: {}", md5);

    let lbhomedir = &state.lbhomedir;

    let installer = PluginInstaller::new(lbhomedir);

    match installer.uninstall(&md5).await {
        Ok(_) => {
            info!("Successfully uninstalled plugin: {}", md5);
            Json(PluginUninstallResponse {
                success: true,
                message: "Plugin uninstalled successfully".to_string(),
            })
            .into_response()
        }
        Err(e) => {
            error!("Failed to uninstall plugin: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PluginUninstallResponse {
                    success: false,
                    message: format!("{}", e),
                }),
            )
                .into_response()
        }
    }
}

/// Upgrade plugin by MD5
///
/// POST /api/plugins/:md5/upgrade
pub async fn upgrade_plugin(
    State(state): State<AppState>,
    Path(md5): Path<String>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    info!("Upgrading plugin: {}", md5);

    // Extract ZIP file from multipart form data
    let mut temp_file: Option<NamedTempFile> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let data = match field.bytes().await {
                Ok(data) => data,
                Err(e) => {
                    error!("Failed to read upload: {}", e);
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(PluginInstallResponse {
                            success: false,
                            plugin: None,
                            error: Some(format!("Failed to read upload: {}", e)),
                        }),
                    )
                        .into_response();
                }
            };

            // Create temp file
            let mut tf = match NamedTempFile::new() {
                Ok(tf) => tf,
                Err(e) => {
                    error!("Failed to create temp file: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(PluginInstallResponse {
                            success: false,
                            plugin: None,
                            error: Some(format!("Failed to create temp file: {}", e)),
                        }),
                    )
                        .into_response();
                }
            };

            // Write data to temp file
            if let Err(e) = tokio::fs::File::from_std(tf.reopen().unwrap())
                .write_all(&data)
                .await
            {
                error!("Failed to write temp file: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(PluginInstallResponse {
                        success: false,
                        plugin: None,
                        error: Some(format!("Failed to write temp file: {}", e)),
                    }),
                )
                    .into_response();
            }

            temp_file = Some(tf);
            break;
        }
    }

    let temp_file = match temp_file {
        Some(tf) => tf,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(PluginInstallResponse {
                    success: false,
                    plugin: None,
                    error: Some("No file uploaded".to_string()),
                }),
            )
                .into_response();
        }
    };

    // Upgrade plugin
    let lbhomedir = &state.lbhomedir;

    let installer = PluginInstaller::new(lbhomedir);

    let request = InstallRequest {
        zip_path: temp_file.path().to_path_buf(),
        action: InstallAction::Upgrade,
        force: false,
    };

    match installer.install(request).await {
        Ok(plugin) => {
            info!("Successfully upgraded plugin: {} v{}", plugin.name, plugin.version);
            (
                StatusCode::OK,
                Json(PluginInstallResponse {
                    success: true,
                    plugin: Some(plugin),
                    error: None,
                }),
            )
                .into_response()
        }
        Err(e) => {
            error!("Failed to upgrade plugin: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PluginInstallResponse {
                    success: false,
                    plugin: None,
                    error: Some(format!("{}", e)),
                }),
            )
                .into_response()
        }
    }
}
