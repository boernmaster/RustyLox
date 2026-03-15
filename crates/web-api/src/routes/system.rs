//! System status routes

use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::state::AppState;

/// Get system status
pub async fn system_status(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let config = state.config.read().await;

    (
        StatusCode::OK,
        Json(json!({
            "status": "running",
            "version": config.base.version,
            "language": config.base.lang,
            "miniserver_count": config.miniserver.len(),
            "mqtt_enabled": config.mqtt.uses_local_broker(),
        })),
    )
}
