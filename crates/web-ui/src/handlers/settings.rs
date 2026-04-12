//! Settings handler

use crate::templates::SettingsTemplate;
use askama::Template;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Form,
};
use serde::Deserialize;
use web_api::AppState;

/// Settings page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let config = state.config.read().await;

    let template = SettingsTemplate {
        lang: config.base.lang.clone(),
        language: config.base.lang.clone(),
        timezone: config.timeserver.timezone.clone(),
        version: state.version.clone(),
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

#[derive(Debug, Deserialize)]
pub struct SettingsFormData {
    pub language: String,
    pub timezone: String,
}

/// Submit settings
pub async fn submit(
    State(state): State<AppState>,
    Form(form): Form<SettingsFormData>,
) -> Response {
    // Get mutable config
    let mut config = state.config.write().await;

    // Update settings
    config.base.lang = form.language;
    config.timeserver.timezone = form.timezone;

    // Save configuration
    match state.config_manager.save_general(&config).await {
        Ok(_) => {
            drop(config); // Release lock
            let _ = state.reload_config().await;
            // Tell HTMX to do a full page reload so the new language takes effect
            // (i18n.js reads document.documentElement.lang which is set server-side).
            let mut headers = HeaderMap::new();
            headers.insert("HX-Refresh", "true".parse().unwrap());
            (StatusCode::OK, headers).into_response()
        }
        Err(e) => Html(format!(
            "<div class='alert alert-danger'>Error saving settings: {}</div>",
            e
        ))
        .into_response(),
    }
}
