//! Plugin web interface serving
//!
//! Serves static files and PHP pages from plugin web directories:
//! - Public:        /plugins/:name/*path  -> webfrontend/html/plugins/:name/
//! - Authenticated: /admin/plugins/:name/*path -> webfrontend/htmlauth/plugins/:name/

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{debug, error, warn};
use web_api::AppState;

/// Allowed file extensions for serving
const ALLOWED_EXTENSIONS: &[(&str, &str)] = &[
    ("html", "text/html; charset=utf-8"),
    ("htm", "text/html; charset=utf-8"),
    ("css", "text/css; charset=utf-8"),
    ("js", "application/javascript; charset=utf-8"),
    ("json", "application/json"),
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("svg", "image/svg+xml"),
    ("ico", "image/x-icon"),
    ("txt", "text/plain; charset=utf-8"),
    ("xml", "application/xml"),
    ("woff", "font/woff"),
    ("woff2", "font/woff2"),
    ("php", "text/html; charset=utf-8"), // Handled specially
];

/// Serve public plugin web interface (no authentication required)
///
/// GET /plugins/:name/*path
pub async fn serve_plugin_public(
    State(state): State<AppState>,
    Path((plugin_name, path)): Path<(String, String)>,
) -> Response {
    let base_dir = state
        .lbhomedir
        .join("webfrontend/html/plugins")
        .join(&plugin_name);

    serve_plugin_file(&base_dir, &path, &plugin_name).await
}

/// Serve public plugin index (no path specified)
///
/// GET /plugins/:name/
pub async fn serve_plugin_public_index(
    State(state): State<AppState>,
    Path(plugin_name): Path<String>,
) -> Response {
    let base_dir = state
        .lbhomedir
        .join("webfrontend/html/plugins")
        .join(&plugin_name);

    serve_plugin_file(&base_dir, "index.html", &plugin_name).await
}

/// Serve authenticated plugin web interface
///
/// GET /admin/plugins/:name/*path
pub async fn serve_plugin_auth(
    State(state): State<AppState>,
    Path((plugin_name, path)): Path<(String, String)>,
) -> Response {
    let base_dir = state
        .lbhomedir
        .join("webfrontend/htmlauth/plugins")
        .join(&plugin_name);

    serve_plugin_file(&base_dir, &path, &plugin_name).await
}

/// Serve authenticated plugin index
///
/// GET /admin/plugins/:name/
pub async fn serve_plugin_auth_index(
    State(state): State<AppState>,
    Path(plugin_name): Path<String>,
) -> Response {
    let base_dir = state
        .lbhomedir
        .join("webfrontend/htmlauth/plugins")
        .join(&plugin_name);

    // Try index.php first (most LoxBerry plugins use PHP), then index.html
    if base_dir.join("index.php").exists() {
        serve_plugin_file(&base_dir, "index.php", &plugin_name).await
    } else {
        serve_plugin_file(&base_dir, "index.html", &plugin_name).await
    }
}

/// Core file serving logic with path traversal protection
async fn serve_plugin_file(base_dir: &PathBuf, path: &str, plugin_name: &str) -> Response {
    // Normalize path - strip leading slashes
    let clean_path = path.trim_start_matches('/');

    // Default to index.html if path is empty
    let file_name = if clean_path.is_empty() {
        "index.html".to_string()
    } else {
        clean_path.to_string()
    };

    let file_path = base_dir.join(&file_name);

    // Check base directory exists
    if !base_dir.exists() {
        warn!("Plugin web directory not found: {}", base_dir.display());
        return not_found_response(&format!(
            "Plugin '{}' has no web interface",
            plugin_name
        ));
    }

    // Security: resolve the path and ensure it stays within the base directory
    // Use lexical path normalization to avoid path traversal
    let resolved = match resolve_safe_path(base_dir, &file_path) {
        Some(p) => p,
        None => {
            warn!(
                "Path traversal attempt for plugin {}: {}",
                plugin_name, path
            );
            return error_response(StatusCode::FORBIDDEN, "Access denied");
        }
    };

    if !resolved.exists() {
        return not_found_response(&format!("File not found: {}", file_name));
    }

    // Determine file extension
    let ext = resolved
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Handle PHP files
    if ext == "php" {
        return serve_php_file(&resolved, plugin_name).await;
    }

    // Find content type for allowed extensions
    let content_type = ALLOWED_EXTENSIONS
        .iter()
        .find(|(e, _)| *e == ext)
        .map(|(_, ct)| *ct);

    match content_type {
        Some(ct) => serve_static_file(&resolved, ct).await,
        None => {
            warn!("Unsupported file type '{}' for plugin {}", ext, plugin_name);
            error_response(StatusCode::FORBIDDEN, "File type not allowed")
        }
    }
}

/// Serve a static file
async fn serve_static_file(path: &PathBuf, content_type: &str) -> Response {
    match tokio::fs::read(path).await {
        Ok(content) => {
            debug!("Serving static file: {}", path.display());
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .body(Body::from(content))
                .unwrap_or_else(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "Build error"))
        }
        Err(e) => {
            error!("Failed to read file {}: {}", path.display(), e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file")
        }
    }
}

/// Execute a PHP script via php-cli and serve the output
async fn serve_php_file(path: &PathBuf, plugin_name: &str) -> Response {
    debug!("Executing PHP file: {}", path.display());

    let output = Command::new("php")
        .arg(path)
        .env("LBHOMEDIR", "/opt/loxberry")
        .env("LBPPLUGINDIR", plugin_name)
        .env("REDIRECT_STATUS", "200")
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            let body = out.stdout;
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(Body::from(body))
                .unwrap_or_else(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "Build error"))
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            error!("PHP execution failed for plugin {}: {}", plugin_name, stderr);
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "PHP execution failed",
            )
        }
        Err(e) => {
            // php-cli may not be installed
            warn!(
                "Failed to execute PHP for plugin {} (is php installed?): {}",
                plugin_name, e
            );
            error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "PHP runtime not available",
            )
        }
    }
}

/// Lexical path safety check - ensures the resolved path is within the base directory
fn resolve_safe_path(base: &PathBuf, path: &PathBuf) -> Option<PathBuf> {
    // Normalize the path lexically to remove ".." components
    let mut normalized = PathBuf::new();
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::ParentDir => {
                // Refuse any ".." traversal
                return None;
            }
            Component::CurDir => {} // Skip "."
            other => normalized.push(other),
        }
    }

    // Verify the normalized path starts with the base directory
    if normalized.starts_with(base) {
        Some(normalized)
    } else {
        None
    }
}

fn not_found_response(message: &str) -> Response {
    error_response(StatusCode::NOT_FOUND, message)
}

fn error_response(status: StatusCode, message: &str) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(message.to_string()))
        .unwrap_or_else(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, "Error").into_response()
        })
}
