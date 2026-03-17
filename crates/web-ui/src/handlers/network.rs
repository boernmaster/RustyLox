//! Network diagnostics UI handler

use crate::templates::NetworkTemplate;
use askama::Template;
use axum::{extract::State, response::Html};
use web_api::AppState;

/// Network diagnostics page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let template = NetworkTemplate {
        version: state.version.clone(),
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}
