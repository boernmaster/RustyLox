//! Health check routes

use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::AppState;

/// Simple health check endpoint (fast, no component checks)
pub async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let mqtt_connected = state
        .mqtt_gateway
        .as_ref()
        .map(|g| g.status().connected)
        .unwrap_or(false);

    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "service": "rustylox",
            "version": state.version,
            "mqtt_connected": mqtt_connected,
        })),
    )
}
