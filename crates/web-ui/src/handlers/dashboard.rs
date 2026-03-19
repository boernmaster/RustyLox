//! Dashboard handler

use crate::templates::{DashboardTemplate, SystemStatus};
use askama::Template;
use axum::{extract::State, response::Html};
use web_api::AppState;

/// Read system uptime from /proc/uptime and format as human-readable string.
fn read_uptime() -> String {
    if let Ok(content) = std::fs::read_to_string("/proc/uptime") {
        if let Some(secs_str) = content.split_whitespace().next() {
            if let Ok(total_secs) = secs_str.parse::<f64>() {
                let total = total_secs as u64;
                let days = total / 86400;
                let hours = (total % 86400) / 3600;
                let mins = (total % 3600) / 60;
                let secs = total % 60;
                return if days > 0 {
                    format!("{}d {}h {}m {}s", days, hours, mins, secs)
                } else if hours > 0 {
                    format!("{}h {}m {}s", hours, mins, secs)
                } else {
                    format!("{}m {}s", mins, secs)
                };
            }
        }
    }
    "Unknown".to_string()
}

/// Dashboard index page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let config = state.config.read().await;

    // Get system status
    let system_status = SystemStatus {
        version: state.version.clone(),
        status: "running".to_string(),
        uptime: read_uptime(),
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
