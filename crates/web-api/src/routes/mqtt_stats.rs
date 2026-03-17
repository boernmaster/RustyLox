//! MQTT Gateway statistics API routes

use axum::{extract::State, Json};
use mqtt_gateway::{RejectedParam, StatsSnapshot};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use crate::AppState;

/// Get MQTT Gateway statistics
pub async fn get_stats(State(state): State<AppState>) -> Json<StatsResponse> {
    let mqtt_gateway = state
        .mqtt_gateway
        .as_ref()
        .expect("MQTT Gateway not initialized");
    let stats = mqtt_gateway.stats();
    let snapshot = stats.snapshot();

    Json(StatsResponse {
        messages_received: snapshot.messages_received,
        messages_relayed: snapshot.messages_relayed,
        messages_filtered: snapshot.messages_filtered,
        miniserver_accepted: snapshot.miniserver_accepted,
        miniserver_rejected: snapshot.miniserver_rejected,
        success_rate: snapshot.success_rate(),
        messages_per_second: snapshot.messages_per_second(),
        uptime_seconds: snapshot.uptime_seconds,
    })
}

/// Get top rejected parameters
pub async fn get_rejected_params(State(state): State<AppState>) -> Json<RejectedParamsResponse> {
    let mqtt_gateway = state
        .mqtt_gateway
        .as_ref()
        .expect("MQTT Gateway not initialized");
    let stats = mqtt_gateway.stats();
    let top_rejected = stats.top_rejected(50); // Get top 50

    let params: Vec<RejectedParamInfo> = top_rejected
        .into_iter()
        .map(|(name, param)| RejectedParamInfo {
            parameter_name: name,
            count: param.count,
            last_value: param.last_value,
            last_seen: param
                .last_seen
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        })
        .collect();

    Json(RejectedParamsResponse { rejected: params })
}

/// Reset statistics
pub async fn reset_stats(State(state): State<AppState>) -> Json<StatusMessage> {
    let mqtt_gateway = state
        .mqtt_gateway
        .as_ref()
        .expect("MQTT Gateway not initialized");
    let stats = mqtt_gateway.stats();
    stats.reset();

    Json(StatusMessage {
        success: true,
        message: "Statistics reset successfully".to_string(),
    })
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub messages_received: u64,
    pub messages_relayed: u64,
    pub messages_filtered: u64,
    pub miniserver_accepted: u64,
    pub miniserver_rejected: u64,
    pub success_rate: f64,
    pub messages_per_second: f64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct RejectedParamsResponse {
    pub rejected: Vec<RejectedParamInfo>,
}

#[derive(Debug, Serialize)]
pub struct RejectedParamInfo {
    pub parameter_name: String,
    pub count: u64,
    pub last_value: String,
    pub last_seen: u64, // Unix timestamp
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusMessage {
    pub success: bool,
    pub message: String,
}
