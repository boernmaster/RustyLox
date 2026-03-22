//! Virtual Input HTTP endpoint
//!
//! Receives data from Loxone Miniserver Virtual HTTP Outputs.
//!
//! In Loxone Config the user creates a **Virtual Output** with an address like:
//! ```text
//! http://<RustyLox-IP>:8080
//! ```
//! and adds **Virtual Output Commands** whose "Command on" field is:
//! ```text
//! /dev/sps/io/<VIname>/<v>
//! ```
//! where `<VIname>` is a freely chosen name and `<v>` is the value (or the
//! placeholder `<v>` which Loxone replaces at runtime).
//!
//! This module exposes the `/dev/sps/io/:name/:value` route that accepts those
//! requests, publishes the data to the MQTT gateway pipeline (so transformers
//! and MQTT publishing apply), and feeds the Miniserver communication monitor.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::state::AppState;
use crate::MiniserverEvent;

/// Optional query parameters (Loxone sometimes appends extra params)
#[derive(Debug, Deserialize, Default)]
pub struct VirtualInputQuery {
    /// Alternate value via query string (?value=123)
    pub value: Option<String>,
}

/// Handle `/dev/sps/io/:name/:value`
///
/// This is the primary endpoint. The Miniserver calls it with the virtual
/// input name and value embedded in the URL path.
pub async fn receive_value(
    State(state): State<AppState>,
    Path((name, value)): Path<(String, String)>,
) -> impl IntoResponse {
    process_virtual_input(&state, &name, &value).await
}

/// Handle `/dev/sps/io/:name` (value in query string or empty)
///
/// Some Miniserver configurations send the value as a query parameter
/// or as a pulse (no value).
pub async fn receive_name_only(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<VirtualInputQuery>,
) -> impl IntoResponse {
    let value = query.value.unwrap_or_default();
    process_virtual_input(&state, &name, &value).await
}

/// Core logic: accept the virtual input, forward to MQTT gateway, and emit
/// a monitor event.
async fn process_virtual_input(state: &AppState, name: &str, value: &str) -> (StatusCode, String) {
    info!("Virtual HTTP Input received: {} = {}", name, value);

    // Emit monitor event so it appears in the real-time monitor UI
    let _ = state.miniserver_monitor.send(MiniserverEvent {
        miniserver_id: 0,
        miniserver_name: "Virtual HTTP Input".to_string(),
        direction: "received".to_string(),
        protocol: "http".to_string(),
        url: Some(format!("/dev/sps/io/{}/{}", name, value)),
        params: Some(format!("{}={}", name, value)),
        response: None,
        code: Some("200".to_string()),
        error: None,
        timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    });

    // Forward to MQTT gateway if available.
    // Convert underscores back to slashes for the MQTT topic
    // (Loxone virtual input names use underscores where MQTT uses slashes).
    let topic = name.replace('_', "/");

    if let Some(gw) = &state.mqtt_gateway {
        let tx = gw.message_sender();
        let msg = mqtt_gateway::GatewayMessage::UdpReceived {
            topic: topic.clone(),
            value: value.to_string(),
        };
        if let Err(e) = tx.send(msg) {
            warn!("Failed to forward virtual input to MQTT gateway: {}", e);
        } else {
            debug!(
                "Virtual input forwarded to MQTT gateway: {} = {}",
                topic, value
            );
        }
    }

    // Return Loxone-compatible XML response (matches Miniserver response format)
    (
        StatusCode::OK,
        format!(
            "<LL control=\"dev/sps/io/{}/{}\" value=\"{}\" Code=\"200\"/>\n",
            name, value, value
        ),
    )
}
