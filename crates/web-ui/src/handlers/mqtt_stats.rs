//! MQTT Statistics page handler

use askama::Template;
use axum::{extract::State, response::Html};
use std::time::SystemTime;
use web_api::AppState;

#[derive(Template)]
#[template(path = "mqtt/stats.html")]
pub struct MqttStatsTemplate {
    pub version: String,
    pub messages_received: u64,
    pub messages_relayed: u64,
    pub messages_filtered: u64,
    pub miniserver_accepted: u64,
    pub miniserver_rejected: u64,
    pub success_rate: String,
    pub success_rate_value: f64, // For numeric comparisons in template
    pub messages_per_second: String,
    pub uptime: String,
    pub rejected_params: Vec<RejectedParamDisplay>,
}

#[derive(Debug, Clone)]
pub struct RejectedParamDisplay {
    pub parameter_name: String,
    pub count: u64,
    pub last_value: String,
    pub time_ago: String,
}

/// GET /mqtt/stats - MQTT statistics page
pub async fn stats(State(state): State<AppState>) -> Html<String> {
    let mqtt_gateway = state
        .mqtt_gateway
        .as_ref()
        .expect("MQTT Gateway not initialized");
    let stats = mqtt_gateway.stats();
    let snapshot = stats.snapshot();
    let rejected = stats.top_rejected(50);

    // Format rejected params for display
    let rejected_params: Vec<RejectedParamDisplay> = rejected
        .into_iter()
        .map(|(name, param)| {
            let time_ago = format_time_ago(param.last_seen);
            RejectedParamDisplay {
                parameter_name: name,
                count: param.count,
                last_value: param.last_value,
                time_ago,
            }
        })
        .collect();

    let success_rate = snapshot.success_rate();
    let template = MqttStatsTemplate {
        version: state.version.clone(),
        messages_received: snapshot.messages_received,
        messages_relayed: snapshot.messages_relayed,
        messages_filtered: snapshot.messages_filtered,
        miniserver_accepted: snapshot.miniserver_accepted,
        miniserver_rejected: snapshot.miniserver_rejected,
        success_rate: format!("{:.1}", success_rate),
        success_rate_value: success_rate,
        messages_per_second: format!("{:.2}", snapshot.messages_per_second()),
        uptime: format_uptime(snapshot.uptime_seconds),
        rejected_params,
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// Format uptime in human-readable form
fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let mins = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Format time ago in human-readable form
fn format_time_ago(time: SystemTime) -> String {
    let now = SystemTime::now();
    let duration = now.duration_since(time).unwrap_or_default();
    let seconds = duration.as_secs();

    if seconds < 60 {
        format!("{}s ago", seconds)
    } else if seconds < 3600 {
        format!("{}m ago", seconds / 60)
    } else if seconds < 86400 {
        format!("{}h ago", seconds / 3600)
    } else {
        format!("{}d ago", seconds / 86400)
    }
}
