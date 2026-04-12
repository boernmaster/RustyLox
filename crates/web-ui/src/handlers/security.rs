//! Security settings handler

use askama::Template;
use axum::{extract::State, response::Html};
use web_api::AppState;

#[derive(Template)]
#[template(path = "admin/security.html")]
pub struct SecurityTemplate {
    pub version: String,
    pub lang: String,
}

/// GET /admin/security - security settings page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let config = state.config.read().await;
    let lang = config.base.lang.clone();
    drop(config);
    let template = SecurityTemplate {
        version: state.version.clone(),
        lang,
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}
