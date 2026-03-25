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
            "version": state.version,
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

/// Validate a log level directive string.
/// Accepts simple levels like "debug" or per-component like "web_api=debug,mqtt_gateway=trace"
fn validate_log_directive(directive: &str) -> bool {
    for part in directive.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if part.contains('=') {
            // component=level format
            let (_component, level) = part.split_once('=').unwrap_or(("", ""));
            let level = level.trim();
            if !VALID_LOG_LEVELS.contains(&level) {
                return false;
            }
        } else if !VALID_LOG_LEVELS.contains(&part) {
            return false;
        }
    }
    true
}

/// Set log level at runtime.
/// Accepts simple levels ("debug") or per-component directives ("web_api=debug,mqtt_gateway=trace")
pub async fn set_log_level(
    State(state): State<AppState>,
    Json(body): Json<SetLogLevelRequest>,
) -> (StatusCode, Json<Value>) {
    let directive = body.log_level.to_lowercase();
    let directive = directive.trim();

    if directive.is_empty() || !validate_log_directive(directive) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!(
                    "Invalid log directive '{}'. Use simple levels (error/warn/info/debug/trace) \
                     or per-component format (web_api=debug,mqtt_gateway=trace).",
                    directive
                )
            })),
        );
    }

    let mut current = state.log_level.write().await;
    *current = directive.to_string();

    std::env::set_var("RUST_LOG", directive);

    tracing::info!("Log level changed to: {}", directive);

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "log_level": directive,
            "message": "Log level updated."
        })),
    )
}
