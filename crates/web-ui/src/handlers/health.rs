//! System health dashboard handler

use askama::Template;
use axum::{extract::State, response::Html};
use web_api::AppState;

#[derive(Template)]
#[template(path = "health.html")]
pub struct HealthTemplate {
    pub version: String,
}

/// GET /health - system health dashboard
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let template = HealthTemplate {
        version: state.version.clone(),
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}
