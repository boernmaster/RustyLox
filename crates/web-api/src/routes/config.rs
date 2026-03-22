//! Configuration routes

use axum::{extract::State, http::StatusCode, Json};
use rustylox_config::GeneralConfig;
use serde_json::{json, Value};

use crate::state::AppState;

/// Get general configuration
pub async fn get_general(State(state): State<AppState>) -> (StatusCode, Json<GeneralConfig>) {
    let config = state.config.read().await;
    (StatusCode::OK, Json(config.clone()))
}

/// Update general configuration
pub async fn update_general(
    State(state): State<AppState>,
    Json(new_config): Json<GeneralConfig>,
) -> (StatusCode, Json<Value>) {
    // Update in-memory config
    {
        let mut config = state.config.write().await;
        *config = new_config.clone();
    }

    // Save to disk
    match state.config_manager.save_general(&new_config).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "Configuration updated successfully"
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": format!("Failed to save configuration: {}", e)
            })),
        ),
    }
}
