//! System update UI handler

use crate::templates::SystemUpdateTemplate;
use askama::Template;
use axum::{extract::State, response::Html};
use web_api::AppState;

/// System update page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let lang = state.config.read().await.base.lang.clone();
    let template = SystemUpdateTemplate {
        version: state.version.clone(),
        lang,
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}
