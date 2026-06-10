//! Router-level tests for the public plugin web routes and the auth boundary.
//!
//! These pin two behaviors that are easy to break silently:
//! - /plugins/:name/* (webfrontend/html) is served WITHOUT authentication, so
//!   the Loxone Miniserver can poll plugin endpoints without cookies
//! - the static admin routes under /plugins (list, install, details, uninstall)
//!   stay behind the auth redirect even though they share the /plugins prefix

use auth::{AuditLogger, AuthService, AuthStore};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use rustylox_config::{ConfigManager, GeneralConfig};
use std::path::Path;
use tower::ServiceExt;
use web_api::AppState;

fn state_with_auth(lbhomedir: &Path) -> AppState {
    // JwtConfig::default() panics without JWT_SECRET (>= 32 bytes)
    std::env::set_var("JWT_SECRET", "integration-test-secret-0123456789abcdef");
    let store = AuthStore::new(&lbhomedir.join("data"));
    let audit = AuditLogger::new(&lbhomedir.join("log"));
    AppState::new(
        lbhomedir.to_path_buf(),
        "test".to_string(),
        ConfigManager::new(lbhomedir.join("config")),
        GeneralConfig::default(),
        None,
    )
    .with_auth(AuthService::new(store, audit))
}

async fn write_plugin_file(lbhomedir: &Path, plugin: &str, rel: &str, content: &str) {
    let path = lbhomedir
        .join("webfrontend/html/plugins")
        .join(plugin)
        .join(rel);
    tokio::fs::create_dir_all(path.parent().expect("file path has a parent"))
        .await
        .expect("create plugin dir");
    tokio::fs::write(&path, content).await.expect("write file");
}

fn perl_available() -> bool {
    std::process::Command::new("perl")
        .arg("-v")
        .output()
        .is_ok()
}

#[tokio::test]
async fn public_plugin_file_is_served_without_auth() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_plugin_file(tmp.path(), "testplug", "css/style.css", "body{color:red}").await;
    let app = web_ui::create_ui_router(state_with_auth(tmp.path()));

    let res = app
        .oneshot(
            Request::builder()
                .uri("/plugins/testplug/css/style.css")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(res.status(), StatusCode::OK);
    let content_type = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(content_type.starts_with("text/css"), "{}", content_type);
    let body = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .expect("body");
    assert_eq!(&body[..], b"body{color:red}");
}

#[tokio::test]
async fn public_plugin_index_falls_back_to_index_html() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_plugin_file(tmp.path(), "testplug", "index.html", "<h1>plugin home</h1>").await;
    let app = web_ui::create_ui_router(state_with_auth(tmp.path()));

    let res = app
        .oneshot(
            Request::builder()
                .uri("/plugins/testplug/")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .expect("body");
    assert_eq!(&body[..], b"<h1>plugin home</h1>");
}

#[tokio::test]
async fn admin_plugin_routes_still_require_auth() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = web_ui::create_ui_router(state_with_auth(tmp.path()));

    // Plugin list page
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/plugins")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::SEE_OTHER);

    // Uninstall shares the /plugins/<segment>/<segment> shape with the public
    // wildcard but must NOT be reachable without auth (static beats wildcard)
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/plugins/abc123/uninstall")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn path_traversal_is_rejected() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write_plugin_file(tmp.path(), "testplug", "index.html", "ok").await;
    tokio::fs::write(tmp.path().join("webfrontend/html/plugins/secret.txt"), "x")
        .await
        .expect("write secret");
    let app = web_ui::create_ui_router(state_with_auth(tmp.path()));

    for uri in [
        "/plugins/testplug/../secret.txt",
        "/plugins/testplug/%2e%2e/secret.txt",
        "/plugins/testplug/sub/%2e%2e/%2e%2e/secret.txt",
    ] {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(res.status(), StatusCode::FORBIDDEN, "uri: {}", uri);
    }
}

#[tokio::test]
async fn cgi_script_receives_query_string_and_request_uri() {
    if !perl_available() {
        eprintln!("skipping: perl not available");
        return;
    }

    let tmp = tempfile::tempdir().expect("tempdir");
    write_plugin_file(
        tmp.path(),
        "testplug",
        "test.cgi",
        "print \"Content-Type: text/plain\\n\\n\";\nprint \"QS=$ENV{QUERY_STRING}|URI=$ENV{REQUEST_URI}\";\n",
    )
    .await;
    let app = web_ui::create_ui_router(state_with_auth(tmp.path()));

    let res = app
        .oneshot(
            Request::builder()
                .uri("/plugins/testplug/test.cgi?foo=bar&baz=1")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .expect("body");
    assert_eq!(
        &body[..],
        b"QS=foo=bar&baz=1|URI=/plugins/testplug/test.cgi?foo=bar&baz=1" as &[u8]
    );
}

#[tokio::test]
async fn cgi_script_receives_post_body() {
    if !perl_available() {
        eprintln!("skipping: perl not available");
        return;
    }

    let tmp = tempfile::tempdir().expect("tempdir");
    write_plugin_file(
        tmp.path(),
        "testplug",
        "post.cgi",
        "read(STDIN, my $body, $ENV{CONTENT_LENGTH});\nprint \"Content-Type: text/plain\\n\\n\";\nprint \"M=$ENV{REQUEST_METHOD}|B=$body\";\n",
    )
    .await;
    let app = web_ui::create_ui_router(state_with_auth(tmp.path()));

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/plugins/testplug/post.cgi")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("value=42"))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .expect("body");
    assert_eq!(&body[..], b"M=POST|B=value=42" as &[u8]);
}
