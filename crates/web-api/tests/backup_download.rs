//! Downloading a backup must answer every request, also for a file whose name
//! cannot be put into the `Content-Disposition` header.

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
    .with_auth(auth_service);
    (state, tmp)
}

async fn admin_token(state: &AppState) -> String {
    let response = create_router(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"username": "admin", "password": ADMIN_PASSWORD}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    body["access_token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn a_backup_whose_name_does_not_fit_into_a_header_is_refused() {
    let (state, tmp) = test_state().await;
    let backup_dir = backup_manager::backup_dir(tmp.path());
    std::fs::create_dir_all(&backup_dir).unwrap();
    // A line break is not allowed in an HTTP header value
    std::fs::write(backup_dir.join("bad\nname.zip"), b"zip").unwrap();
    let token = admin_token(&state).await;

    let response = create_router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/backup/bad%0Aname.zip/download")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
