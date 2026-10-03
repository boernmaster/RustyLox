//! The API router must reject unauthenticated requests on everything except
//! an explicit allowlist of endpoints that machines without credentials call
//! (health checks, login, addon self-registration, Miniserver callbacks).

use auth::{AuditLogger, AuthService, AuthStore};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::json;
use tower::util::ServiceExt;
use web_api::{create_router, AppState};

const ADMIN_PASSWORD: &str = "test-admin-password";

async fn test_state() -> (AppState, tempfile::TempDir) {
    std::env::set_var("JWT_SECRET", "0123456789abcdef0123456789abcdef");
    std::env::set_var("ADMIN_PASSWORD", ADMIN_PASSWORD);

    let tmp = tempfile::tempdir().unwrap();
    let config_dir = tmp.path().join("config/system");
    let data_dir = tmp.path().join("data/system");
    let log_dir = tmp.path().join("log/system");
    for dir in [&config_dir, &data_dir, &log_dir] {
        std::fs::create_dir_all(dir).unwrap();
    }

    let auth_service = AuthService::new(AuthStore::new(&data_dir), AuditLogger::new(&log_dir));
    auth_service.init().await.unwrap();

    let state = AppState::new(
        tmp.path().to_path_buf(),
        "test".to_string(),
        rustylox_config::ConfigManager::new(&config_dir),
        rustylox_config::GeneralConfig::default(),
        None,
    )
    .with_auth(auth_service)
    .with_addon_registry(
        std::sync::Arc::new(addon_registry::Registry::new()),
        data_dir.join("addonregistry.json"),
    );
    (state, tmp)
}

async fn status_of(state: &AppState, request: Request<Body>) -> StatusCode {
    create_router(state.clone())
        .oneshot(request)
        .await
        .unwrap()
        .status()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn post_json(uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn admin_token(state: &AppState) -> String {
    let response = create_router(state.clone())
        .oneshot(post_json(
            "/api/auth/login",
            json!({"username": "admin", "password": ADMIN_PASSWORD}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    body["access_token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn unauthenticated_read_of_a_previously_open_endpoint_is_rejected() {
    let (state, _tmp) = test_state().await;
    assert_eq!(
        status_of(&state, get("/api/system/log-level")).await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn unauthenticated_miniserver_command_is_rejected() {
    let (state, _tmp) = test_state().await;
    assert_eq!(
        status_of(
            &state,
            post_json("/api/miniserver/1/send", json!({"params": []}))
        )
        .await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn unauthenticated_addon_config_proxy_is_rejected() {
    let (state, _tmp) = test_state().await;
    assert_eq!(
        status_of(
            &state,
            post_json(
                "/api/addons/some-addon/config",
                json!({"MQTT_HOST": "evil"})
            )
        )
        .await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn unauthenticated_prometheus_metrics_are_rejected() {
    let (state, _tmp) = test_state().await;
    assert_eq!(
        status_of(&state, get("/metrics")).await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn health_check_stays_public() {
    let (state, _tmp) = test_state().await;
    assert_eq!(status_of(&state, get("/health")).await, StatusCode::OK);
    assert_eq!(status_of(&state, get("/api/health")).await, StatusCode::OK);
}

#[tokio::test]
async fn addon_self_registration_stays_public() {
    let (state, _tmp) = test_state().await;
    let status = status_of(
        &state,
        post_json(
            "/api/addons/register",
            json!({
                "name": "test-addon",
                "version": "1.0.0",
                "config_api_base_url": "http://127.0.0.1:9"
            }),
        ),
    )
    .await;
    assert_ne!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn miniserver_callbacks_stay_public() {
    let (state, _tmp) = test_state().await;
    assert_ne!(
        status_of(&state, get("/dev/sps/io/some_input/1")).await,
        StatusCode::UNAUTHORIZED
    );
    assert_ne!(
        status_of(&state, get("/forecast/")).await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn bearer_token_from_login_grants_access() {
    let (state, _tmp) = test_state().await;
    let token = admin_token(&state).await;
    let request = Request::builder()
        .uri("/api/system/log-level")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    assert_eq!(status_of(&state, request).await, StatusCode::OK);
}

#[tokio::test]
async fn session_cookie_grants_access() {
    let (state, _tmp) = test_state().await;
    let token = admin_token(&state).await;
    let request = Request::builder()
        .uri("/api/system/log-level")
        .header("cookie", format!("lb_token={token}"))
        .body(Body::empty())
        .unwrap();
    assert_eq!(status_of(&state, request).await, StatusCode::OK);
}
