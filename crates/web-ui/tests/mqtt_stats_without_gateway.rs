//! The MQTT statistics page must render a notice, not panic, when the MQTT
//! gateway is not running (`AppState::mqtt_gateway` is `None`).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rustylox_config::{ConfigManager, GeneralConfig};
use tower::ServiceExt;
use web_api::AppState;

#[tokio::test]
async fn stats_page_explains_that_the_gateway_is_not_running() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let state = AppState::new(
        tmp.path().to_path_buf(),
        "test".to_string(),
        ConfigManager::new(tmp.path().join("config")),
        GeneralConfig::default(),
        None,
    );

    let response = web_ui::create_ui_router(state)
        .oneshot(
            Request::builder()
                .uri("/mqtt/stats")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    assert!(String::from_utf8_lossy(&body).contains("MQTT gateway is not running"));
}
