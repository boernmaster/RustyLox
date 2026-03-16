//! System status routes

use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::state::AppState;

/// Get system status
pub async fn system_status(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let config = state.config.read().await;
    let log_level = state.log_level.read().await;

    (
        StatusCode::OK,
        Json(json!({
            "status": "running",
            "version": config.base.version,
            "language": config.base.lang,
            "miniserver_count": config.miniserver.len(),
            "mqtt_enabled": config.mqtt.uses_local_broker(),
            "log_level": *log_level,
        })),
    )
}

/// Get current log level
pub async fn get_log_level(State(state): State<AppState>) -> Json<Value> {
    let level = state.log_level.read().await;
    Json(json!({ "log_level": *level }))
}

#[derive(Debug, Deserialize)]
pub struct SetLogLevelRequest {
    pub log_level: String,
}

const VALID_LOG_LEVELS: &[&str] = &["error", "warn", "info", "debug", "trace"];

/// Set log level at runtime
pub async fn set_log_level(
    State(state): State<AppState>,
    Json(body): Json<SetLogLevelRequest>,
) -> (StatusCode, Json<Value>) {
    let level = body.log_level.to_lowercase();

    if !VALID_LOG_LEVELS.contains(&level.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!(
                    "Invalid log level '{}'. Valid: error, warn, info, debug, trace",
                    level
                )
            })),
        );
    }

    let mut current = state.log_level.write().await;
    *current = level.clone();

    // Propagate to environment so subprocesses (plugin scripts) inherit it
    std::env::set_var("RUST_LOG", &level);

    tracing::info!("Log level changed to: {}", level);

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "log_level": level,
            "message": "Log level updated."
        })),
    )
}
