//! The MQTT gateway is optional (`AppState::mqtt_gateway` is `None` when it
//! could not be started). The statistics endpoints must report that instead
//! of panicking inside the request handler.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::util::ServiceExt;
use web_api::{create_router, AppState};

/// AppState without an MQTT gateway (and without auth).
fn state_without_gateway() -> (AppState, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let config_dir = tmp.path().join("config/system");
    std::fs::create_dir_all(&config_dir).unwrap();
    let state = AppState::new(
        tmp.path().to_path_buf(),
        "test".to_string(),
        rustylox_config::ConfigManager::new(&config_dir),
        rustylox_config::GeneralConfig::default(),
        None,
    );
    (state, tmp)
}

async fn status_of(method: &str, uri: &str) -> StatusCode {
    let (state, _tmp) = state_without_gateway();
    create_router(state)
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn stats_report_service_unavailable() {
    assert_eq!(
        status_of("GET", "/api/mqtt/stats").await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn rejected_params_report_service_unavailable() {
    assert_eq!(
        status_of("GET", "/api/mqtt/rejected").await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn stats_reset_reports_service_unavailable() {
    assert_eq!(
        status_of("POST", "/api/mqtt/stats/reset").await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}
