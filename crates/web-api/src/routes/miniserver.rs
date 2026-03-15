//! Miniserver control routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::state::AppState;

/// List all configured Miniservers
pub async fn list_miniservers(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let config = state.config.read().await;

    let miniservers: Vec<Value> = config
        .miniserver
        .iter()
        .map(|(id, ms)| {
            json!({
                "id": id,
                "name": ms.name,
                "ipaddress": ms.ipaddress,
                "port": ms.port,
                "transport": ms.transport,
                "useclouddns": ms.useclouddns,
            })
        })
        .collect();

    (StatusCode::OK, Json(json!({ "miniservers": miniservers })))
}

/// Get single Miniserver configuration
pub async fn get_miniserver(
    State(state): State<AppState>,
    Path(id): Path<u8>,
) -> (StatusCode, Json<Value>) {
    let config = state.config.read().await;

    match config.miniserver.get(&id.to_string()) {
        Some(ms) => (
            StatusCode::OK,
            Json(json!({
                "id": id,
                "name": ms.name,
                "ipaddress": ms.ipaddress,
                "port": ms.port,
                "porthttps": ms.porthttps,
                "transport": ms.transport,
                "useclouddns": ms.useclouddns,
                "cloudurl": ms.cloudurl,
            })),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "status": "error",
                "message": format!("Miniserver {} not found", id)
            })),
        ),
    }
}

/// Request body for send command
#[derive(Debug, Deserialize)]
pub struct SendCommandRequest {
    params: Vec<SendParam>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SendParam {
    parameter: String,
    value: String,
}

/// Send command to Miniserver
pub async fn send_command(
    State(state): State<AppState>,
    Path(id): Path<u8>,
    Json(request): Json<SendCommandRequest>,
) -> (StatusCode, Json<Value>) {
    // Get Miniserver client
    let client = match state.get_miniserver_client(id).await {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": format!("Failed to get Miniserver client: {}", e)
                })),
            )
        }
    };

    // Convert params to vector of tuples
    let params: Vec<(String, String)> = request
        .params
        .into_iter()
        .map(|p| (p.parameter, p.value))
        .collect();

    // Send command
    match client.send(params).await {
        Ok(results) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "results": results
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": format!("Failed to send command: {}", e)
            })),
        ),
    }
}

/// Request body for get values
#[derive(Debug, Deserialize)]
pub struct GetValuesRequest {
    params: Vec<String>,
}

/// Get values from Miniserver
pub async fn get_values(
    State(state): State<AppState>,
    Path(id): Path<u8>,
    Json(request): Json<GetValuesRequest>,
) -> (StatusCode, Json<Value>) {
    // Get Miniserver client
    let client = match state.get_miniserver_client(id).await {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": format!("Failed to get Miniserver client: {}", e)
                })),
            )
        }
    };

    // Get values
    match client.get(request.params).await {
        Ok(values) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "values": values
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": format!("Failed to get values: {}", e)
            })),
        ),
    }
}

/// Check Miniserver connection status
pub async fn check_status(
    State(state): State<AppState>,
    Path(id): Path<u8>,
) -> (StatusCode, Json<Value>) {
    // Get Miniserver client
    let client = match state.get_miniserver_client(id).await {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": format!("Failed to get Miniserver client: {}", e)
                })),
            )
        }
    };

    // Try a simple call to /dev/lan/txp to check connectivity
    match client.http().call("/dev/lan/txp").await {
        Ok((value, code, _)) => (
            StatusCode::OK,
            Json(json!({
                "status": "online",
                "connected": true,
                "txp_value": value,
                "response_code": code
            })),
        ),
        Err(e) => (
            StatusCode::OK, // Still return 200, but indicate offline status
            Json(json!({
                "status": "offline",
                "connected": false,
                "error": e.to_string()
            })),
        ),
    }
}
