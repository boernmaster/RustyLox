//! System health dashboard handler

use askama::Template;
use axum::{extract::State, response::Html};
use web_api::AppState;

#[derive(Template)]
#[template(path = "health.html")]
pub struct HealthTemplate {
    pub version: String,
    pub lang: String,
}

/// GET /system-health - system health dashboard
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let lang = state.config.read().await.base.lang.clone();
    let template = HealthTemplate {
        version: state.version.clone(),
        lang,
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}
