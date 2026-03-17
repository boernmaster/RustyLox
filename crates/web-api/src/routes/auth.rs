//! Authentication routes: login, logout, user management, API key management

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use auth::{Action, AuthIdentity, AuthService, Resource, Role};

use crate::state::AppState;

// ─── Request/Response types ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub email: String,
    pub roles: Vec<Role>,
}

#[derive(Serialize)]
pub struct UserSummary {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub roles: Vec<Role>,
    pub enabled: bool,
    pub last_login: Option<DateTime<Utc>>,
    pub is_locked: bool,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub new_password: String,
}

#[derive(Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub key: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ApiKeySummary {
    pub id: Uuid,
    pub name: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// ─── Helper functions ─────────────────────────────────────────────────────────

fn auth_error(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({"error": msg})))
}

pub fn extract_ip(headers: &HeaderMap) -> String {
    headers
        .get("X-Real-IP")
        .or_else(|| headers.get("X-Forwarded-For"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .split(',')
        .next()
        .unwrap_or("unknown")
        .trim()
        .to_string()
}

pub async fn extract_identity(
    headers: &HeaderMap,
    service: &AuthService,
) -> Result<AuthIdentity, (StatusCode, Json<serde_json::Value>)> {
    // Try Bearer token first
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(value) = auth_header.to_str() {
            if let Some(token) = value.strip_prefix("Bearer ") {
                return service
                    .authenticate_token(token)
                    .await
                    .map_err(|e| auth_error(StatusCode::UNAUTHORIZED, &e.to_string()));
            }
        }
    }

    // Try API key header
    if let Some(api_key_header) = headers.get("X-API-Key") {
        if let Ok(key) = api_key_header.to_str() {
            return service
                .authenticate_api_key(key)
                .await
                .map_err(|e| auth_error(StatusCode::UNAUTHORIZED, &e.to_string()));
        }
    }

    Err(auth_error(
        StatusCode::UNAUTHORIZED,
        "Authentication required",
    ))
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// POST /api/auth/login
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let Some(service) = &state.auth_service else {
        return auth_error(StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response();
    };
    let ip = extract_ip(&headers);
    match service.login(&req.username, &req.password, &ip).await {
        Ok(token_response) => (
            StatusCode::OK,
            Json(serde_json::to_value(&token_response).unwrap()),
        )
            .into_response(),
        Err(e) => auth_error(
            StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::UNAUTHORIZED),
            &e.to_string(),
        )
        .into_response(),
    }
}

/// POST /api/auth/logout
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let Some(service) = &state.auth_service else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(value) = auth_header.to_str() {
            if let Some(token) = value.strip_prefix("Bearer ") {
                if let Ok(claims) = auth::jwt::validate_token(&service.jwt_config, token) {
                    service.sessions.remove(&claims.jti);
                }
            }
        }
    }
    (
        StatusCode::OK,
        Json(serde_json::json!({"message": "Logged out"})),
    )
        .into_response()
}

/// GET /api/auth/me
pub async fn me(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let Some(service) = &state.auth_service else {
        return auth_error(StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response();
    };
    match extract_identity(&headers, service).await {
        Ok(identity) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "user_id": identity.user_id,
                "username": identity.username,
                "roles": identity.roles,
            })),
        )
            .into_response(),
        Err(e) => e.into_response(),
    }
}

/// GET /api/users
pub async fn list_users(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let Some(service) = &state.auth_service else {
        return auth_error(StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response();
    };
    let identity = match extract_identity(&headers, service).await {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };
    if !identity.can(&Resource::Users, &Action::Read) {
        return auth_error(StatusCode::FORBIDDEN, "Insufficient permissions").into_response();
    }
    match service.store.list_users().await {
        Ok(users) => {
            let summaries: Vec<UserSummary> = users
                .into_iter()
                .map(|u| {
                    let locked = u.is_locked();
                    UserSummary {
                        id: u.id,
                        username: u.username,
                        email: u.email,
                        roles: u.roles,
                        enabled: u.enabled,
                        last_login: u.last_login,
                        is_locked: locked,
                    }
                })
                .collect();
            (
                StatusCode::OK,
                Json(serde_json::to_value(&summaries).unwrap()),
            )
                .into_response()
        }
        Err(e) => auth_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

/// POST /api/users
pub async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateUserRequest>,
) -> impl IntoResponse {
    let Some(service) = &state.auth_service else {
        return auth_error(StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response();
    };
    let identity = match extract_identity(&headers, service).await {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };
    let ip = extract_ip(&headers);
    match service
        .create_user(
            &identity,
            req.username,
            req.password,
            req.email,
            req.roles,
            &ip,
        )
        .await
    {
        Ok(user) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "id": user.id,
                "username": user.username,
            })),
        )
            .into_response(),
        Err(e) => auth_error(
            StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::BAD_REQUEST),
            &e.to_string(),
        )
        .into_response(),
    }
}

/// DELETE /api/users/:id
pub async fn delete_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    let Some(service) = &state.auth_service else {
        return auth_error(StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response();
    };
    let identity = match extract_identity(&headers, service).await {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };
    let ip = extract_ip(&headers);
    match service.delete_user(&identity, &user_id, &ip).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => auth_error(
            StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::BAD_REQUEST),
            &e.to_string(),
        )
        .into_response(),
    }
}

/// PUT /api/users/:id/password
pub async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(req): Json<ChangePasswordRequest>,
) -> impl IntoResponse {
    let Some(service) = &state.auth_service else {
        return auth_error(StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response();
    };
    let identity = match extract_identity(&headers, service).await {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };
    let ip = extract_ip(&headers);
    match service
        .change_password(&identity, &user_id, &req.new_password, &ip)
        .await
    {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"message": "Password updated"})),
        )
            .into_response(),
        Err(e) => auth_error(
            StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::BAD_REQUEST),
            &e.to_string(),
        )
        .into_response(),
    }
}

/// GET /api/auth/keys
pub async fn list_api_keys(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let Some(service) = &state.auth_service else {
        return auth_error(StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response();
    };
    let identity = match extract_identity(&headers, service).await {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };
    match service
        .store
        .list_api_keys_for_user(&identity.user_id)
        .await
    {
        Ok(keys) => {
            let summaries: Vec<ApiKeySummary> = keys
                .into_iter()
                .map(|k| ApiKeySummary {
                    id: k.id,
                    name: k.name,
                    expires_at: k.expires_at,
                    last_used: k.last_used,
                    created_at: k.created_at,
                })
                .collect();
            (
                StatusCode::OK,
                Json(serde_json::to_value(&summaries).unwrap()),
            )
                .into_response()
        }
        Err(e) => auth_error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

/// POST /api/auth/keys
pub async fn create_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateApiKeyRequest>,
) -> impl IntoResponse {
    let Some(service) = &state.auth_service else {
        return auth_error(StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response();
    };
    let identity = match extract_identity(&headers, service).await {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };
    let ip = extract_ip(&headers);
    // Grant all read permissions by default for new API keys
    let permissions = vec![
        (Resource::Plugins, Action::Read),
        (Resource::Settings, Action::Read),
        (Resource::Miniserver, Action::Read),
    ];
    match service
        .create_api_key(&identity, req.name, permissions, req.expires_at, &ip)
        .await
    {
        Ok((key, raw_key)) => (
            StatusCode::CREATED,
            Json(
                serde_json::to_value(CreateApiKeyResponse {
                    id: key.id,
                    name: key.name,
                    key: raw_key,
                    expires_at: key.expires_at,
                    created_at: key.created_at,
                })
                .unwrap(),
            ),
        )
            .into_response(),
        Err(e) => auth_error(
            StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::BAD_REQUEST),
            &e.to_string(),
        )
        .into_response(),
    }
}

/// DELETE /api/auth/keys/:id
pub async fn delete_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key_id): Path<Uuid>,
) -> impl IntoResponse {
    let Some(service) = &state.auth_service else {
        return auth_error(StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response();
    };
    let identity = match extract_identity(&headers, service).await {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };
    let ip = extract_ip(&headers);
    match service.delete_api_key(&identity, &key_id, &ip).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => auth_error(
            StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::BAD_REQUEST),
            &e.to_string(),
        )
        .into_response(),
    }
}

/// GET /api/auth/audit
pub async fn get_audit_log(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let Some(service) = &state.auth_service else {
        return auth_error(StatusCode::SERVICE_UNAVAILABLE, "Auth not configured").into_response();
    };
    let identity = match extract_identity(&headers, service).await {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };
    if !identity.is_admin() {
        return auth_error(StatusCode::FORBIDDEN, "Admin required").into_response();
    }
    let entries = service.audit.read_recent(100).await;
    (
        StatusCode::OK,
        Json(serde_json::to_value(&entries).unwrap()),
    )
        .into_response()
}
