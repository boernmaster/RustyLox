//! Pages served by the UI router carry the browser security headers, the same
//! way the API responses do.

use axum::body::Body;
use axum::http::{HeaderMap, Request};
use rustylox_config::{ConfigManager, GeneralConfig};
use std::path::Path;
use tower::ServiceExt;
use web_api::AppState;

fn state(lbhomedir: &Path) -> AppState {
    AppState::new(
        lbhomedir.to_path_buf(),
        "test".to_string(),
        ConfigManager::new(lbhomedir.join("config")),
        GeneralConfig::default(),
        None,
    )
}

async fn response_headers(lbhomedir: &Path, uri: &str) -> HeaderMap {
    web_ui::create_ui_router(state(lbhomedir))
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
        .headers()
        .clone()
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> &'a str {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
}

#[tokio::test]
async fn ui_pages_may_only_be_framed_by_rustylox_itself() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let headers = response_headers(tmp.path(), "/login").await;

    assert_eq!(header(&headers, "x-frame-options"), "SAMEORIGIN");
    assert!(
        header(&headers, "content-security-policy").contains("frame-ancestors 'self'"),
        "{headers:?}"
    );
}

#[tokio::test]
async fn ui_pages_forbid_content_type_sniffing() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let headers = response_headers(tmp.path(), "/login").await;

    assert_eq!(header(&headers, "x-content-type-options"), "nosniff");
}

#[tokio::test]
async fn plugin_pages_carry_the_security_headers_too() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let plugin_dir = tmp.path().join("webfrontend/html/plugins/testplug");
    tokio::fs::create_dir_all(&plugin_dir)
        .await
        .expect("create plugin dir");
    tokio::fs::write(plugin_dir.join("index.html"), "<h1>plugin</h1>")
        .await
        .expect("write file");

    let headers = response_headers(tmp.path(), "/plugins/testplug/index.html").await;

    assert_eq!(header(&headers, "x-frame-options"), "SAMEORIGIN");
    assert_eq!(header(&headers, "x-content-type-options"), "nosniff");
}
