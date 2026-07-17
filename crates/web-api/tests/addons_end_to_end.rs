//! Capstone end-to-end integration test for the containerized-addons feature.
//!
//! Spins up a fake addon HTTP server (mimicking kia-connect-bridge's config
//! API) and drives it through the *real* Axum router: register -> list ->
//! schema proxy -> config save proxy. Proves Tasks 6/7 work together, not
//! just in isolation.

use addon_registry::Registry;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use http_body_util::BodyExt;
use serde_json::json;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tower::util::ServiceExt;
use web_api::{create_router, AppState};

/// Spawns a minimal fake addon server that answers the same three endpoints
/// a real containerized addon (e.g. kia-connect-bridge) exposes. Returns its
/// base URL (e.g. "http://127.0.0.1:54321") plus a handle to the JSON body
/// most recently received by the `save` (config POST) handler, so tests can
/// assert the proxy forwarded the payload byte-for-byte rather than just
/// checking the fake server returned 200.
async fn spawn_fake_addon() -> (String, Arc<Mutex<Option<serde_json::Value>>>) {
    async fn schema() -> Json<serde_json::Value> {
        Json(
            json!([{"key": "MQTT_HOST", "label": "MQTT Host", "type": "text", "help": "", "secret": false}]),
        )
    }
    async fn config() -> Json<serde_json::Value> {
        Json(json!({"MQTT_HOST": {"value": "10.0.0.32", "secret_set": false}}))
    }

    let captured_body: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    let captured_body_for_handler = captured_body.clone();
    let save = move |Json(body): Json<serde_json::Value>| {
        let captured_body = captured_body_for_handler.clone();
        async move {
            *captured_body.lock().unwrap() = Some(body);
            Json(json!({"saved": true}))
        }
    };

    let app = Router::new()
        .route("/addon/schema", get(schema))
        .route("/addon/config", get(config).post(save));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{}", addr), captured_body)
}

fn test_state(registry: Arc<Registry>) -> AppState {
    let tmp = PathBuf::from("/tmp/rustylox-e2e-test");
    let config_dir = tmp.join("config/system");
    std::fs::create_dir_all(&config_dir).ok();

    let config_manager = rustylox_config::ConfigManager::new(&config_dir);
    let config = rustylox_config::GeneralConfig::default();

    AppState::new(
        tmp.clone(),
        "test".to_string(),
        config_manager,
        config,
        None,
    )
    .with_addon_registry(registry, tmp.join("data/system/addonregistry.json"))
}

async fn body_string(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

#[tokio::test]
async fn full_register_discover_configure_loop() {
    let (fake_addon_url, captured_save_body) = spawn_fake_addon().await;
    let registry = Arc::new(Registry::new());
    let app = create_router(test_state(registry));

    // 1. Addon self-registers
    let register_body = json!({
        "name": "kia-connect-bridge",
        "version": "1.0.0",
        "config_api_base_url": fake_addon_url
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

    // 2. RustyLox lists it as online
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/addons")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_string(response).await;
    let instances: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    assert_eq!(instances[0]["name"], "kia-connect-bridge");
    assert_eq!(instances[0]["online"], true);

    // 3. RustyLox fetches its schema through the proxy
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/addons/kia-connect-bridge/schema")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;
    let schema: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(schema[0]["key"], "MQTT_HOST");

    // 4. RustyLox saves a config change through the proxy
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/addons/kia-connect-bridge/config")
                .header("content-type", "application/json")
                .body(Body::from(json!({"MQTT_HOST": "10.0.0.99"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // The fake addon must have received exactly the payload we sent, proving
    // proxy::save_config forwards the real body (not empty/wrong/mangled).
    let received_body = captured_save_body.lock().unwrap().clone();
    assert_eq!(received_body, Some(json!({"MQTT_HOST": "10.0.0.99"})));
}
