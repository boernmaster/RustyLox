//! Addon registration and listing routes.
//!
//! Containerized addons (e.g. kia-connect-bridge) self-register via a
//! periodic heartbeat POST to `/api/addons/register`. This endpoint is
//! deliberately unauthenticated - same LAN-trust model as the rest of
//! RustyLox (plain HTTP, not internet-facing).

use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use chrono::Utc;

use addon_registry::proxy;
use addon_registry::{AddonInstance, RegisterRequest};
use tracing::{error, warn};

use crate::state::AppState;

/// POST /api/addons/register
pub async fn register(
    State(state): State<AppState>,
    body: Result<Json<RegisterRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    let Json(request) = match body {
        Ok(json) => json,
        Err(e) => {
            warn!("Rejected malformed addon registration: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::json!({ "error": format!("Invalid registration payload: {}", e) }),
                ),
            )
                .into_response();
        }
    };

    if request.name.trim().is_empty() || request.config_api_base_url.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "name and config_api_base_url are required" })),
        )
            .into_response();
    }

    let Some(registry) = &state.addon_registry else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Addon registry not configured" })),
        )
            .into_response();
    };

    let instance = AddonInstance::from_request(request, Utc::now());
    registry.register(instance).await;

    if let Some(path) = &state.addon_registry_path {
        if let Err(e) = registry.save(path).await {
            error!("Failed to persist addon registry: {}", e);
        }
    }

    StatusCode::CREATED.into_response()
}

/// GET /api/addons
pub async fn list(State(state): State<AppState>) -> impl IntoResponse {
    let Some(registry) = &state.addon_registry else {
        return Json(Vec::<addon_registry::AddonInstanceView>::new()).into_response();
    };
    let views = registry.list(Utc::now()).await;
    Json(views).into_response()
}

/// GET /api/addons/:name/schema
pub async fn schema(State(state): State<AppState>, Path(name): Path<String>) -> impl IntoResponse {
    proxy_get(&state, &name, proxy::fetch_schema).await
}

/// GET /api/addons/:name/config
pub async fn config(State(state): State<AppState>, Path(name): Path<String>) -> impl IntoResponse {
    proxy_get(&state, &name, proxy::fetch_config).await
}

async fn proxy_get<F, Fut>(state: &AppState, name: &str, call: F) -> axum::response::Response
where
    F: FnOnce(&str) -> Fut,
    Fut: std::future::Future<Output = Result<serde_json::Value, proxy::ProxyError>>,
{
    let Some(registry) = &state.addon_registry else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Addon registry not configured",
        )
            .into_response();
    };
    let Some(instance) = registry.find(name).await else {
        return (StatusCode::NOT_FOUND, format!("Unknown addon: {}", name)).into_response();
    };
    match call(&instance.config_api_base_url).await {
        Ok(value) => Json(value).into_response(),
        Err(e) => {
            warn!("Proxy call to addon '{}' failed: {}", name, e);
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": "addon offline" })),
            )
                .into_response()
        }
    }
}

/// POST /api/addons/:name/config
pub async fn save_config(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let Some(registry) = &state.addon_registry else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Addon registry not configured",
        )
            .into_response();
    };
    let Some(instance) = registry.find(&name).await else {
        return (StatusCode::NOT_FOUND, format!("Unknown addon: {}", name)).into_response();
    };
    match proxy::save_config(&instance.config_api_base_url, &payload).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => {
            warn!("Proxy save to addon '{}' failed: {}", name, e);
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": "addon offline" })),
            )
                .into_response()
        }
    }
}
