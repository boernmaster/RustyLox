//! Authentication and admin UI handlers

use crate::templates::{AdminApiKeysTemplate, AdminAuditTemplate, AdminUsersTemplate, LoginTemplate};
use askama::Template;
use axum::{
    extract::{Form, Query, State},
    response::{Html, Redirect},
};
use serde::Deserialize;
use web_api::AppState;

#[derive(Deserialize)]
pub struct LoginQuery {
    pub error: Option<String>,
    pub redirect: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
    pub remember_me: Option<String>,
}

/// GET /login - show login form
pub async fn show_login(
    State(state): State<AppState>,
    Query(params): Query<LoginQuery>,
) -> Html<String> {
    let template = LoginTemplate {
        error: params.error,
        version: state.version.clone(),
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// POST /login - handle login form submission
pub async fn handle_login(
    State(state): State<AppState>,
    Form(creds): Form<LoginForm>,
) -> Redirect {
    // If auth service is not configured, redirect to dashboard
    let Some(auth_service) = &state.auth_service else {
        return Redirect::to("/");
    };

    match auth_service
        .login(&creds.username, &creds.password, "web-form")
        .await
    {
        Ok(_token_response) => Redirect::to("/"),
        Err(e) => {
            let msg = format!("{}", e);
            let encoded = urlencoding_encode(&msg);
            Redirect::to(&format!("/login?error={}", encoded))
        }
    }
}

/// GET /admin/users - user management page
pub async fn users(State(state): State<AppState>) -> Html<String> {
    let template = AdminUsersTemplate {
        version: state.version.clone(),
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// GET /admin/api-keys - API key management page
pub async fn api_keys(State(state): State<AppState>) -> Html<String> {
    let template = AdminApiKeysTemplate {
        version: state.version.clone(),
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// GET /admin/audit - audit log viewer
pub async fn audit_log(State(state): State<AppState>) -> Html<String> {
    let template = AdminAuditTemplate {
        version: state.version.clone(),
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// Simple percent-encoding for error messages in query params
fn urlencoding_encode(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                vec![c]
            }
            c => {
                let encoded = format!("%{:02X}", c as u32);
                encoded.chars().collect()
            }
        })
        .collect()
}
