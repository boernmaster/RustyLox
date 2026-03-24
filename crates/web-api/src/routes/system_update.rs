//! System update API endpoints - check and apply updates from GitHub releases

use crate::state::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

/// GitHub release information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub html_url: String,
    pub published_at: String,
    pub prerelease: bool,
    pub assets: Vec<ReleaseAsset>,
}

/// GitHub release asset (downloadable file)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

/// Response for the update check endpoint
#[derive(Debug, Serialize)]
pub struct UpdateCheckResponse {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_name: String,
    pub release_notes: String,
    pub release_url: String,
    pub published_at: String,
    pub prerelease: bool,
    pub assets: Vec<ReleaseAsset>,
}

const GITHUB_REPO: &str = "boernmaster/RustyLox";

/// Check for available updates by querying GitHub releases
///
/// GET /api/system/update/check
pub async fn check_update(State(state): State<AppState>) -> impl IntoResponse {
    let current_version = state.version.clone();
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let client = match reqwest::Client::builder()
        .user_agent("RustyLox-Updater")
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create HTTP client: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("HTTP client error: {}", e) })),
            )
                .into_response();
        }
    };

    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to reach GitHub API: {}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": format!("Cannot reach GitHub: {}", e) })),
            )
                .into_response();
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        warn!("GitHub API returned {}: {}", status, body);
        return (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "error": format!("GitHub API returned {}", status)
            })),
        )
            .into_response();
    }

    let release: ReleaseInfo = match response.json().await {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to parse GitHub release: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to parse release: {}", e) })),
            )
                .into_response();
        }
    };

    let latest = release.tag_name.trim_start_matches('v');
    let current = current_version.trim_start_matches('v');
    // Simple string comparison; works for semver when format is consistent
    let update_available = latest != current && latest > current;

    info!(
        "Update check: current={}, latest={}, available={}",
        current, latest, update_available
    );

    Json(UpdateCheckResponse {
        current_version: current_version,
        latest_version: release.tag_name.clone(),
        update_available,
        release_name: release.name,
        release_notes: release.body,
        release_url: release.html_url,
        published_at: release.published_at,
        prerelease: release.prerelease,
        assets: release.assets,
    })
    .into_response()
}

/// Apply a system update (downloads and replaces the binary)
///
/// POST /api/system/update/apply
///
/// For Docker deployments, this signals that the user should pull the new image.
/// For binary deployments, this could download and replace the binary.
pub async fn apply_update(State(state): State<AppState>) -> impl IntoResponse {
    // In a Docker-first deployment, "applying" an update means pulling the new image.
    // We provide instructions rather than doing it automatically for safety.
    info!("System update apply requested");

    let config = state.config.read().await;
    let install_type = config.update.installtype.clone();
    drop(config);

    let message = if install_type == "docker" || install_type.is_empty() {
        "To update RustyLox, pull the latest Docker image:\n\n\
         docker compose pull\n\
         docker compose up -d\n\n\
         The system will restart automatically with the new version."
            .to_string()
    } else {
        "To update RustyLox, download the latest release from GitHub and replace the binary.\n\
         Restart the service after updating."
            .to_string()
    };

    Json(serde_json::json!({
        "success": true,
        "install_type": install_type,
        "message": message
    }))
    .into_response()
}
