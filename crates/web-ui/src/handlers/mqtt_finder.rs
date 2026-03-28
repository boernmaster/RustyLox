//! MQTT Finder page handler

use crate::templates::MqttFinderTemplate;
use askama::Template;
use axum::{extract::State, response::Html};
use web_api::AppState;

/// GET /mqtt/finder - MQTT Finder page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let template = MqttFinderTemplate {
        title: "MQTT Finder".to_string(),
        version: state.version.clone(),
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}
