//! Authentication and admin UI handlers

use crate::templates::{
    AdminApiKeysTemplate, AdminAuditTemplate, AdminUsersTemplate, LoginTemplate,
};
use askama::Template;
use axum::{
    extract::{Form, Query, State},
    http::{header, HeaderValue},
    response::{Html, IntoResponse, Redirect, Response},
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
) -> Response {
    // If auth service is not configured, redirect to dashboard
    let Some(auth_service) = &state.auth_service else {
        return Redirect::to("/").into_response();
    };

    match auth_service
        .login(&creds.username, &creds.password, "web-form")
        .await
    {
        Ok(token_response) => {
            // 6 months = 6 * 30 * 24 * 3600 = 15_552_000 seconds
            let cookie = format!(
                "lb_token={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=15552000",
                token_response.access_token
            );
            let mut response = Redirect::to("/").into_response();
            if let Ok(cookie_value) = HeaderValue::from_str(&cookie) {
                response
                    .headers_mut()
                    .insert(header::SET_COOKIE, cookie_value);
            }
            response
        }
        Err(e) => {
            let msg = format!("{}", e);
            let encoded = urlencoding_encode(&msg);
            Redirect::to(&format!("/login?error={}", encoded)).into_response()
        }
    }
}

/// POST /logout - clear session cookie and redirect to login
pub async fn handle_logout() -> Response {
    let clear_cookie = "lb_token=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0";
    let mut response = Redirect::to("/login").into_response();
    if let Ok(cookie_value) = HeaderValue::from_str(clear_cookie) {
        response
            .headers_mut()
            .insert(header::SET_COOKIE, cookie_value);
    }
    response
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
