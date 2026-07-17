//! Integration tests for the `/api/addons/register` and `/api/addons` routes.

use addon_registry::Registry;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::path::PathBuf;
use std::sync::Arc;
use tower::util::ServiceExt;
use web_api::{create_router, AppState};

fn test_state(registry: Arc<Registry>) -> AppState {
    let tmp = PathBuf::from("/tmp/rustylox-test-addons");
    let config_dir = tmp.join("config/system");
    std::fs::create_dir_all(&config_dir).ok();

    let config_manager = rustylox_config::ConfigManager::new(&config_dir);
    let config = rustylox_config::GeneralConfig::default();

    AppState::new(tmp.clone(), "test".to_string(), config_manager, config, None)
        .with_addon_registry(registry, tmp.join("data/system/addonregistry.json"))
}

/// Helper: read a response body to a String (matches virtual_input_test.rs's convention).
async fn body_string(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

#[tokio::test]
async fn register_then_list_round_trips() {
    let registry = Arc::new(Registry::new());
    let app = create_router(test_state(registry));

    let register_body = serde_json::json!({
        "name": "kia-connect-bridge",
        "version": "1.0.0",
        "config_api_base_url": "http://10.0.0.32:8090"
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/addons/register")
                .header("content-type", "application/json")
                .body(Body::from(register_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/addons")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response).await;
    let instances: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0]["name"], "kia-connect-bridge");
    assert_eq!(instances[0]["online"], true);
}

#[tokio::test]
async fn register_rejects_malformed_payload() {
    let registry = Arc::new(Registry::new());
    let app = create_router(test_state(registry));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/addons/register")
                .header("content-type", "application/json")
                .body(Body::from("{\"name\": \"missing fields\"}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
