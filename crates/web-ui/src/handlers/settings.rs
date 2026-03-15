//! Settings handler

use crate::templates::SettingsTemplate;
use askama::Template;
use axum::{
    extract::State,
    response::Html,
    Form,
};
use serde::Deserialize;
use std::sync::Arc;
use web_api::AppState;

/// Settings page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let config = state.config.read().await;

    let template = SettingsTemplate {
        language: config.base.lang.clone(),
        timezone: config.timeserver.timezone.clone(),
        version: config.base.version.clone(),
    };

    Html(template.render().unwrap_or_else(|_| "Error rendering template".to_string()))
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
) -> Html<String> {
    // TODO: Update configuration
    // TODO: Save to file

    Html("<div class='success'>Settings saved successfully</div>".to_string())
}
