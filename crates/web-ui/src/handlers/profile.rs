//! User profile handler

use askama::Template;
use axum::{extract::State, response::Html};
use web_api::AppState;

#[derive(Template)]
#[template(path = "profile.html")]
pub struct ProfileTemplate {
    pub version: String,
}

/// GET /profile - current user profile page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let template = ProfileTemplate {
        version: state.version.clone(),
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}
