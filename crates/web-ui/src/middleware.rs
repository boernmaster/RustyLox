//! Auth middleware - redirects unauthenticated requests to /login

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use web_api::AppState;

/// Paths that are accessible without authentication
fn is_public_path(path: &str) -> bool {
    path == "/login"
        || path == "/logout"
        || path.starts_with("/static/")
        || path == "/health"
        || path == "/api/health"
}

/// Extract the lb_token cookie value from the Cookie header
fn extract_lb_token(cookie_header: &str) -> Option<&str> {
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some(token) = part.strip_prefix("lb_token=") {
            if !token.is_empty() {
                return Some(token);
            }
        }
    }
    None
}

/// Middleware that enforces authentication for all non-public routes.
/// Redirects unauthenticated users to /login?redirect=<original_path>.
pub async fn require_auth(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let path = request.uri().path();

    // Allow public paths through without auth check
    if is_public_path(path) {
        return next.run(request).await;
    }

    // If no auth service is configured, allow all requests
    let Some(auth_service) = &state.auth_service else {
        return next.run(request).await;
    };

    // Try to extract and validate the lb_token cookie
    let authenticated = if let Some(cookie_header) = request.headers().get("Cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            if let Some(token) = extract_lb_token(cookie_str) {
                auth_service.authenticate_token(token).await.is_ok()
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    if authenticated {
        return next.run(request).await;
    }

    // Try HTTP Basic Auth: accept any password that is a valid API key (lbx_...)
    // This allows the Loxone Miniserver VirtualOut to authenticate using a URL like:
    //   http://admin:lbx_TOKEN@10.0.0.7/admin/plugins/...
    let authenticated_via_api_key =
        if let Some(auth_header) = request.headers().get("Authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if let Some(encoded) = auth_str.strip_prefix("Basic ") {
                    if let Ok(decoded) = BASE64.decode(encoded.trim()) {
                        if let Ok(credentials) = std::str::from_utf8(&decoded) {
                            // credentials = "username:password" — the password is the API key
                            let password = credentials.splitn(2, ':').nth(1).unwrap_or("");
                            if password.starts_with("lbx_") {
                                auth_service.authenticate_api_key(password).await.is_ok()
                            } else {
                                // Fall back to regular username:password verification
                                let username = credentials.splitn(2, ':').next().unwrap_or("");
                                auth_service
                                    .verify_user_password(username, password)
                                    .await
                                    .is_ok()
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

    if authenticated_via_api_key {
        return next.run(request).await;
    }

    // Not authenticated - redirect to login with the original path as redirect param
    let redirect_path = {
        let uri = request.uri();
        match uri.query() {
            Some(q) => format!("{}?{}", uri.path(), q),
            None => uri.path().to_string(),
        }
    };
    let encoded = percent_encode(&redirect_path);
    Redirect::to(&format!("/login?redirect={}", encoded)).into_response()
}

/// Minimal percent-encoding for use in query parameter values
fn percent_encode(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' | '/' => vec![c],
            c => format!("%{:02X}", c as u32).chars().collect(),
        })
        .collect()
}
