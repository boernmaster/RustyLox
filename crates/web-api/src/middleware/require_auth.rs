//! Authentication middleware for the API router
//!
//! Every API route requires a valid identity (Bearer token, `X-API-Key` or the
//! `lb_token` session cookie) unless it is on the explicit allowlist below.
//! Handlers may still call `extract_identity` themselves to get at the
//! identity for permission checks; this layer only guarantees that nothing is
//! reachable anonymously by accident.

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::routes::auth::extract_identity;
use crate::AppState;

/// Endpoints called by machines that have no credentials.
fn is_public_path(path: &str) -> bool {
    matches!(
        path,
        // Container/orchestrator health checks
        "/health" | "/api/health"
        // Obtaining a token in the first place
        | "/api/auth/login"
        // Containerized addons self-register (LAN-trust model)
        | "/api/addons/register"
        // Loxone Cloud Emulator, polled by the Miniserver
        | "/forecast" | "/forecast/"
    ) || path.starts_with("/dev/sps/io/") // Miniserver Virtual HTTP Outputs
}

/// Rejects unauthenticated requests to non-public API routes with 401.
pub async fn require_auth(State(state): State<AppState>, request: Request, next: Next) -> Response {
    if is_public_path(request.uri().path()) {
        return next.run(request).await;
    }

    // If no auth service is configured, allow all requests (same as the web UI)
    let Some(auth_service) = &state.auth_service else {
        return next.run(request).await;
    };

    match extract_identity(request.headers(), auth_service).await {
        Ok(_) => next.run(request).await,
        Err(rejection) => rejection.into_response(),
    }
}
