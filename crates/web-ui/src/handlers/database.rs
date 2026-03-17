//! Data management handler

use askama::Template;
use axum::{extract::State, response::Html};
use web_api::AppState;

#[derive(Template)]
#[template(path = "admin/database.html")]
pub struct DatabaseTemplate {
    pub version: String,
}

/// GET /admin/database - data management page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let template = DatabaseTemplate {
        version: state.version.clone(),
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}
