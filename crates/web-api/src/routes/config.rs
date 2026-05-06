//! Configuration routes

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use rustylox_config::GeneralConfig;
use serde_json::json;

use crate::routes::auth::extract_identity;
use crate::state::AppState;

/// Get general configuration
pub async fn get_general(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let Some(service) = &state.auth_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Auth not configured"})),
        )
            .into_response();
    };
    if let Err(e) = extract_identity(&headers, service).await {
        return e.into_response();
    }
    let config = state.config.read().await;
    (StatusCode::OK, Json(config.clone())).into_response()
}

/// Update general configuration
pub async fn update_general(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(new_config): Json<GeneralConfig>,
) -> impl IntoResponse {
    let Some(service) = &state.auth_service else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Auth not configured"})),
        )
            .into_response();
    };
    if let Err(e) = extract_identity(&headers, service).await {
        return e.into_response();
    }
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
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": format!("Failed to save configuration: {}", e)
            })),
        )
            .into_response(),
    }
}
