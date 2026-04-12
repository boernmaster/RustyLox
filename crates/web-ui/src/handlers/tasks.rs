//! Scheduled tasks UI handler

use crate::templates::TasksTemplate;
use askama::Template;
use axum::{extract::State, response::Html};
use web_api::AppState;

/// Scheduled tasks page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let lang = state.config.read().await.base.lang.clone();
    let template = TasksTemplate {
        version: state.version.clone(),
        lang,
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}
