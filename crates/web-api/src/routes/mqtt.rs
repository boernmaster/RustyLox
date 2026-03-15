//! MQTT Gateway API endpoints

use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

/// MQTT status response
#[derive(Debug, Serialize)]
pub struct MqttStatusResponse {
    pub connected: bool,
    pub subscriptions: usize,
    pub transformers: usize,
}

/// Publish request
#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    pub topic: String,
    pub payload: String,
}

/// Get MQTT gateway status
///
/// GET /api/mqtt/status
pub async fn get_status(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(gateway) = &state.mqtt_gateway {
        let status = gateway.status();
        Json(status).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "MQTT gateway not running"
            })),
        )
            .into_response()
    }
}

/// Reload subscriptions from disk
///
/// POST /api/mqtt/subscriptions/reload
pub async fn reload_subscriptions(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(gateway) = &state.mqtt_gateway {
        match gateway.reload_subscriptions().await {
            Ok(_) => {
                info!("Subscriptions reloaded successfully");
                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "success": true,
                        "message": "Subscriptions reloaded"
                    })),
                )
            }
            Err(e) => {
                error!("Failed to reload subscriptions: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "success": false,
                        "error": format!("{}", e)
                    })),
                )
            }
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "MQTT gateway not running"
            })),
        )
    }
}

/// Reload transformers from disk
///
/// POST /api/mqtt/transformers/reload
pub async fn reload_transformers(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(gateway) = &state.mqtt_gateway {
        match gateway.reload_transformers().await {
            Ok(_) => {
                info!("Transformers reloaded successfully");
                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "success": true,
                        "message": "Transformers reloaded"
                    })),
                )
            }
            Err(e) => {
                error!("Failed to reload transformers: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "success": false,
                        "error": format!("{}", e)
                    })),
                )
            }
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "MQTT gateway not running"
            })),
        )
    }
}
