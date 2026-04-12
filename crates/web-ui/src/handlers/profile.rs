//! User profile handler

use askama::Template;
use axum::{extract::State, response::Html};
use web_api::AppState;

#[derive(Template)]
#[template(path = "profile.html")]
pub struct ProfileTemplate {
    pub version: String,
    pub lang: String,
}

/// GET /profile - current user profile page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let config = state.config.read().await;
    let lang = config.base.lang.clone();
    drop(config);
    let template = ProfileTemplate {
        version: state.version.clone(),
        lang,
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}
