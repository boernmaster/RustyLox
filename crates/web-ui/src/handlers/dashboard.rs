//! Dashboard handler

use crate::templates::{DashboardTemplate, SystemStatus};
use askama::Template;
use axum::{extract::State, response::Html};
use std::sync::Arc;
use web_api::AppState;

/// Dashboard index page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let config = state.config.read().await;

    // Get system status
    let system_status = SystemStatus {
        version: config.base.version.clone(),
        status: "running".to_string(),
        uptime: "Unknown".to_string(), // TODO: Calculate uptime
    };

    // Get miniserver count
    let miniserver_count = config.miniserver.len();

    // Get plugin count
    let plugin_manager = plugin_manager::PluginInstaller::new(&state.lbhomedir);
    let plugin_count = match plugin_manager.list().await {
        Ok(plugins) => plugins.len(),
        Err(_) => 0,
    };

    // Get MQTT status
    let mqtt_connected = if let Some(gateway) = &state.mqtt_gateway {
        gateway.status().connected
    } else {
        false
    };

    let template = DashboardTemplate {
        system_status,
        miniserver_count,
        plugin_count,
        mqtt_connected,
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}
