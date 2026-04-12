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
    /// Hidden field - original URL to redirect to after login
    pub redirect: Option<String>,
}

/// GET /login - show login form
pub async fn show_login(
    State(state): State<AppState>,
    Query(params): Query<LoginQuery>,
) -> Html<String> {
    let lang = state.config.read().await.base.lang.clone();
    let template = LoginTemplate {
        error: params.error,
        redirect: params.redirect,
        version: state.version.clone(),
        lang,
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// POST /login - handle login form submission
pub async fn handle_login(State(state): State<AppState>, Form(creds): Form<LoginForm>) -> Response {
    // If auth service is not configured, redirect to dashboard
    let Some(auth_service) = &state.auth_service else {
        return Redirect::to("/").into_response();
    };

    match auth_service
        .login(&creds.username, &creds.password, "web-form")
        .await
    {
        Ok(token_response) => {
            // Always set a 30-day persistent cookie (2_592_000 s) so the session
            // survives browser restarts.  The JWT itself also expires after 30 days.
            let _ = creds.remember_me; // field kept for forward-compatibility
            let cookie = format!(
                "lb_token={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=2592000",
                token_response.access_token
            );

            // Redirect to original destination or dashboard
            let destination = creds
                .redirect
                .as_deref()
                .filter(|r| !r.is_empty() && r.starts_with('/'))
                .unwrap_or("/");

            let mut response = Redirect::to(destination).into_response();
            if let Ok(cookie_value) = HeaderValue::from_str(&cookie) {
                response
                    .headers_mut()
                    .insert(header::SET_COOKIE, cookie_value);
            }
            response
        }
        Err(e) => {
            let msg = format!("{}", e);
            let encoded_msg = urlencoding_encode(&msg);
            let redirect_param = creds
                .redirect
                .as_deref()
                .filter(|r| !r.is_empty())
                .map(|r| format!("&redirect={}", urlencoding_encode(r)))
                .unwrap_or_default();
            Redirect::to(&format!("/login?error={}{}", encoded_msg, redirect_param)).into_response()
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
    let lang = state.config.read().await.base.lang.clone();
    let template = AdminUsersTemplate {
        version: state.version.clone(),
        lang,
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// GET /admin/api-keys - API key management page
pub async fn api_keys(State(state): State<AppState>) -> Html<String> {
    let lang = state.config.read().await.base.lang.clone();
    let template = AdminApiKeysTemplate {
        version: state.version.clone(),
        lang,
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// GET /admin/audit - audit log viewer
pub async fn audit_log(State(state): State<AppState>) -> Html<String> {
    let lang = state.config.read().await.base.lang.clone();
    let template = AdminAuditTemplate {
        version: state.version.clone(),
        lang,
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
