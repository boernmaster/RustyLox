//! Security settings handler

use askama::Template;
use axum::{extract::State, response::Html};
use web_api::AppState;

#[derive(Template)]
#[template(path = "admin/security.html")]
pub struct SecurityTemplate {
    pub version: String,
}

/// GET /admin/security - security settings page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let template = SecurityTemplate {
        version: state.version.clone(),
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}
