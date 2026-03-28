//! MQTT Gateway API endpoints

use crate::state::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
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

/// Get relayed topics for the "Incoming Overview" monitor
///
/// GET /api/mqtt/relayed-topics
pub async fn get_relayed_topics(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(gateway) = &state.mqtt_gateway {
        let tracker = gateway.relay_tracker();
        let response = tracker.get_relayed_topics();
        Json(response).into_response()
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

/// Update per-topic settings (disable_cache, reset_after_send, do_not_forward)
///
/// POST /api/mqtt/topic-settings
pub async fn update_topic_setting(
    State(state): State<AppState>,
    Json(body): Json<TopicSettingRequest>,
) -> impl IntoResponse {
    if let Some(gateway) = &state.mqtt_gateway {
        let tracker = gateway.relay_tracker();
        tracker.update_topic_setting(&body.topic, &body.setting, body.enabled);
        info!(
            "Topic setting updated: {} {} = {}",
            body.topic, body.setting, body.enabled
        );
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Topic setting updated"
            })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "MQTT gateway not running"
            })),
        )
    }
}

/// Delete a topic from the relay tracker cache
///
/// POST /api/mqtt/topic-delete
pub async fn delete_topic_cache(
    State(state): State<AppState>,
    Json(body): Json<TopicDeleteRequest>,
) -> impl IntoResponse {
    if let Some(gateway) = &state.mqtt_gateway {
        let tracker = gateway.relay_tracker();
        tracker.delete_topic(&body.topic);
        info!("Topic deleted from cache: {}", body.topic);
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Topic deleted from cache"
            })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "MQTT gateway not running"
            })),
        )
    }
}

/// Clear all relay tracker cache
///
/// POST /api/mqtt/relay-cache/clear
pub async fn clear_relay_cache(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(gateway) = &state.mqtt_gateway {
        let tracker = gateway.relay_tracker();
        tracker.clear();
        info!("Relay cache cleared");
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Relay cache cleared"
            })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "MQTT gateway not running"
            })),
        )
    }
}

#[derive(Debug, Deserialize)]
pub struct TopicSettingRequest {
    pub topic: String,
    pub setting: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct TopicDeleteRequest {
    pub topic: String,
}

/// Get MQTT Finder data (all topics seen on broker)
///
/// GET /api/mqtt/finder
pub async fn get_finder_data(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(gateway) = &state.mqtt_gateway {
        let finder = gateway.mqtt_finder();
        let entries = finder.get_all();
        let total = finder.topic_count();
        let total_messages = finder.total_messages();
        Json(serde_json::json!({
            "topics": entries,
            "topic_count": total,
            "total_messages": total_messages,
        }))
        .into_response()
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
