//! Integration tests for the `/dev/sps/io/` virtual input endpoint.
//!
//! These tests verify that the Miniserver Virtual HTTP Output can send data
//! to RustyLox and that it is correctly processed.

use axum::body::Body;
use http_body_util::BodyExt;
use std::path::PathBuf;
use tower::util::ServiceExt;
use web_api::{create_router, AppState, MiniserverEvent};

/// Create a minimal AppState for testing (no MQTT gateway, no auth).
fn test_state() -> AppState {
    let tmp = PathBuf::from("/tmp/rustylox-test-vi");
    let config_dir = tmp.join("config/system");
    std::fs::create_dir_all(&config_dir).ok();

    let config_manager = rustylox_config::ConfigManager::new(&config_dir);
    let config = rustylox_config::GeneralConfig::default();

    AppState::new(tmp, "test".to_string(), config_manager, config, None)
}

/// Helper: send a GET request to the router and return (status, body_string).
async fn get(app: axum::Router, uri: &str) -> (u16, String) {
    let req = axum::http::Request::builder()
        .uri(uri)
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status().as_u16();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&body).to_string();

    (status, text)
}

// ─── /dev/sps/io/:name/:value ────────────────────────────────────────────────

#[tokio::test]
async fn test_virtual_input_returns_loxone_xml() {
    let state = test_state();
    let app = create_router(state);

    let (status, body) = get(app, "/dev/sps/io/TestVI/42").await;

    assert_eq!(status, 200);
    assert!(
        body.contains("Code=\"200\""),
        "Response should contain Loxone XML Code attribute, got: {}",
        body
    );
    assert!(
        body.contains("value=\"42\""),
        "Response should echo the value, got: {}",
        body
    );
}

#[tokio::test]
async fn test_virtual_input_name_only_no_value() {
    let state = test_state();
    let app = create_router(state);

    let (status, body) = get(app, "/dev/sps/io/PulseInput").await;

    assert_eq!(status, 200);
    assert!(body.contains("Code=\"200\""));
}

#[tokio::test]
async fn test_virtual_input_name_with_query_value() {
    let state = test_state();
    let app = create_router(state);

    let (status, body) = get(app, "/dev/sps/io/Sensor?value=99.5").await;

    assert_eq!(status, 200);
    assert!(body.contains("value=\"99.5\""), "got: {}", body);
}

#[tokio::test]
async fn test_virtual_input_emits_monitor_event() {
    let state = test_state();
    let mut rx = state.miniserver_monitor.subscribe();
    let app = create_router(state);

    let _resp = get(app, "/dev/sps/io/Temp_Outdoor/18.3").await;

    // The monitor channel should have received an event
    let event: MiniserverEvent = rx
        .try_recv()
        .expect("Monitor channel should have received an event");

    assert_eq!(event.direction, "received");
    assert_eq!(event.protocol, "http");
    assert_eq!(event.miniserver_name, "Virtual HTTP Input");
    assert!(
        event
            .params
            .as_deref()
            .unwrap()
            .contains("Temp_Outdoor=18.3"),
        "Params should contain name=value, got: {:?}",
        event.params
    );
}

#[tokio::test]
async fn test_virtual_input_url_encoded_value() {
    let state = test_state();
    let app = create_router(state);

    // Loxone sometimes sends URL-encoded values
    let (status, body) = get(app, "/dev/sps/io/Text_Status/Hello%20World").await;

    assert_eq!(status, 200);
    assert!(body.contains("Code=\"200\""));
}

#[tokio::test]
async fn test_virtual_input_multiple_requests() {
    let state = test_state();
    let mut rx = state.miniserver_monitor.subscribe();
    let app = create_router(state.clone());

    // First request
    let (s1, _) = get(app, "/dev/sps/io/V1/100").await;
    assert_eq!(s1, 200);

    // Second request (new router instance needed since oneshot consumes it)
    let app2 = create_router(state);
    let (s2, _) = get(app2, "/dev/sps/io/V2/0").await;
    assert_eq!(s2, 200);

    // Both events should be in the monitor
    let e1 = rx.try_recv().expect("First event missing");
    let e2 = rx.try_recv().expect("Second event missing");

    assert!(e1.params.as_deref().unwrap().contains("V1=100"));
    assert!(e2.params.as_deref().unwrap().contains("V2=0"));
}
