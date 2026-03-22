//! Plugin web interface serving
//!
//! Serves static files and PHP pages from plugin web directories:
//! - Public:        /plugins/:name/*path  -> webfrontend/html/plugins/:name/
//! - Authenticated: /admin/plugins/:name/*path -> webfrontend/htmlauth/plugins/:name/

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
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

/// PHP CGI request context
struct PhpRequest {
    method: String,
    query_string: String,
    content_type: String,
    body: Vec<u8>,
}

impl PhpRequest {
    fn get_only() -> Self {
        Self {
            method: "GET".to_string(),
            query_string: String::new(),
            content_type: String::new(),
            body: Vec::new(),
        }
    }
}

/// Serve public plugin web interface (no authentication required)
///
/// GET /plugins/web/:name/*path
pub async fn serve_plugin_public(
    State(state): State<AppState>,
    Path((plugin_name, path)): Path<(String, String)>,
) -> Response {
    let base_dir = state
        .lbhomedir
        .join("webfrontend/html/plugins")
        .join(&plugin_name);

    serve_plugin_file(&base_dir, &path, &plugin_name, PhpRequest::get_only()).await
}

/// Serve public plugin index (no path specified)
///
/// GET /plugins/web/:name/
pub async fn serve_plugin_public_index(
    State(state): State<AppState>,
    Path(plugin_name): Path<String>,
) -> Response {
    let base_dir = state
        .lbhomedir
        .join("webfrontend/html/plugins")
        .join(&plugin_name);

    serve_plugin_file(
        &base_dir,
        "index.html",
        &plugin_name,
        PhpRequest::get_only(),
    )
    .await
}

/// Serve authenticated plugin web interface (GET)
///
/// GET /admin/plugins/:name/*path
pub async fn serve_plugin_auth(
    State(state): State<AppState>,
    Path((plugin_name, path)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    let base_dir = state
        .lbhomedir
        .join("webfrontend/htmlauth/plugins")
        .join(&plugin_name);

    let php_req = PhpRequest {
        method: "GET".to_string(),
        query_string: uri.query().unwrap_or("").to_string(),
        content_type: String::new(),
        body: Vec::new(),
    };

    serve_plugin_file(&base_dir, &path, &plugin_name, php_req).await
}

/// Serve authenticated plugin web interface (POST for AJAX)
///
/// POST /admin/plugins/:name/*path
pub async fn serve_plugin_auth_post(
    State(state): State<AppState>,
    Path((plugin_name, path)): Path<(String, String)>,
    uri: Uri,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let base_dir = state
        .lbhomedir
        .join("webfrontend/htmlauth/plugins")
        .join(&plugin_name);

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let php_req = PhpRequest {
        method: "POST".to_string(),
        query_string: uri.query().unwrap_or("").to_string(),
        content_type,
        body: body.to_vec(),
    };

    serve_plugin_file(&base_dir, &path, &plugin_name, php_req).await
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

    // Try index.php → index.cgi → index.html
    if base_dir.join("index.php").exists() {
        serve_plugin_file(&base_dir, "index.php", &plugin_name, PhpRequest::get_only()).await
    } else if base_dir.join("index.cgi").exists() {
        serve_plugin_file(&base_dir, "index.cgi", &plugin_name, PhpRequest::get_only()).await
    } else {
        serve_plugin_file(
            &base_dir,
            "index.html",
            &plugin_name,
            PhpRequest::get_only(),
        )
        .await
    }
}

/// Core file serving logic with path traversal protection
async fn serve_plugin_file(
    base_dir: &PathBuf,
    path: &str,
    plugin_name: &str,
    php_req: PhpRequest,
) -> Response {
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
        return not_found_response(&format!("Plugin '{}' has no web interface", plugin_name));
    }

    // Security: resolve the path and ensure it stays within the base directory
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
        return serve_php_file(&resolved, plugin_name, php_req).await;
    }

    // Handle Perl CGI files
    if ext == "cgi" {
        return serve_cgi_file(&resolved, plugin_name, php_req).await;
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
                .unwrap_or_else(|_| {
                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Build error")
                })
        }
        Err(e) => {
            error!("Failed to read file {}: {}", path.display(), e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file")
        }
    }
}

/// Execute a PHP script via php-cli with CGI environment and serve the output
async fn serve_php_file(path: &PathBuf, plugin_name: &str, php_req: PhpRequest) -> Response {
    debug!(
        "Executing PHP file: {} (method={}, query={})",
        path.display(),
        php_req.method,
        php_req.query_string
    );

    // Set PHP include_path and auto-prepend bootstrap for PHP 8.x compatibility
    let include_path = ".:/opt/loxberry/libs/phplib:/usr/share/php";
    let bootstrap = "/opt/loxberry/libs/phplib/loxberry_bootstrap.php";

    let mut cmd = Command::new("php-cgi");
    cmd.arg("-d")
        .arg(format!("include_path={}", include_path))
        .arg("-d")
        .arg(format!("auto_prepend_file={}", bootstrap))
        .arg(path)
        // Run from script's directory so relative includes (e.g. "./phpMQTT/...") work
        .current_dir(path.parent().unwrap_or(path))
        // LoxBerry environment
        .env("LBHOMEDIR", "/opt/loxberry")
        .env("LBPPLUGINDIR", plugin_name)
        .env(
            "LBPHTMLDIR",
            format!("/opt/loxberry/webfrontend/html/plugins/{}", plugin_name),
        )
        .env(
            "LBPHTMLAUTHDIR",
            format!("/opt/loxberry/webfrontend/htmlauth/plugins/{}", plugin_name),
        )
        .env(
            "LBPDATADIR",
            format!("/opt/loxberry/data/plugins/{}", plugin_name),
        )
        .env(
            "LBPLOGDIR",
            format!("/opt/loxberry/log/plugins/{}", plugin_name),
        )
        .env(
            "LBPCONFIGDIR",
            format!("/opt/loxberry/config/plugins/{}", plugin_name),
        )
        // CGI environment variables for $_GET, $_POST, $_SERVER
        .env("REDIRECT_STATUS", "200")
        .env("REQUEST_METHOD", &php_req.method)
        .env("QUERY_STRING", &php_req.query_string)
        .env("CONTENT_TYPE", &php_req.content_type)
        .env("CONTENT_LENGTH", php_req.body.len().to_string())
        .env("SCRIPT_FILENAME", path.to_string_lossy().to_string())
        .env("SERVER_PROTOCOL", "HTTP/1.1")
        .env("GATEWAY_INTERFACE", "CGI/1.1")
        .env("SERVER_SOFTWARE", "RustyLox");

    // For POST requests, pipe the body via stdin and capture stdout
    if !php_req.body.is_empty() {
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
    }

    let output = if php_req.body.is_empty() {
        cmd.output().await
    } else {
        // Spawn and write body to stdin
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to spawn PHP for plugin {}: {}", plugin_name, e);
                return error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "PHP runtime not available (php-cgi)",
                );
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.write_all(&php_req.body).await;
            drop(stdin);
        }

        child.wait_with_output().await
    };

    match output {
        Ok(out) => {
            let stdout = out.stdout;
            let stderr = String::from_utf8_lossy(&out.stderr);
            if !stderr.is_empty() {
                debug!("PHP stderr for plugin {}: {}", plugin_name, stderr);
            }

            // Parse CGI headers from output (Content-Type, Status, etc.)
            let (status_code, headers, body) = parse_php_cgi_output(&stdout);

            let content_type = headers
                .get("content-type")
                .cloned()
                .unwrap_or_else(|| "text/html; charset=utf-8".to_string());

            let mut builder = Response::builder().status(status_code);
            builder = builder.header(header::CONTENT_TYPE, &content_type);

            // Forward other headers from PHP
            for (key, value) in &headers {
                if key != "content-type" && key != "status" {
                    builder = builder.header(key.as_str(), value.as_str());
                }
            }

            builder.body(Body::from(body)).unwrap_or_else(|_| {
                error_response(StatusCode::INTERNAL_SERVER_ERROR, "Build error")
            })
        }
        Err(e) => {
            warn!(
                "Failed to execute PHP for plugin {} (is php installed?): {}",
                plugin_name, e
            );
            error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "PHP runtime not available (php-cgi)",
            )
        }
    }
}

/// Execute a Perl CGI script and serve the output
async fn serve_cgi_file(path: &PathBuf, plugin_name: &str, php_req: PhpRequest) -> Response {
    debug!(
        "Executing CGI file: {} (method={}, query={})",
        path.display(),
        php_req.method,
        php_req.query_string
    );

    let mut cmd = Command::new("perl");
    cmd.arg(path)
        .current_dir(path.parent().unwrap_or(path))
        .env("PERL5LIB", "/opt/loxberry/libs/perllib")
        .env("LBHOMEDIR", "/opt/loxberry")
        .env("LBPPLUGINDIR", plugin_name)
        .env(
            "LBPHTMLDIR",
            format!("/opt/loxberry/webfrontend/html/plugins/{}", plugin_name),
        )
        .env(
            "LBPHTMLAUTHDIR",
            format!("/opt/loxberry/webfrontend/htmlauth/plugins/{}", plugin_name),
        )
        .env(
            "LBPDATADIR",
            format!("/opt/loxberry/data/plugins/{}", plugin_name),
        )
        .env(
            "LBPLOGDIR",
            format!("/opt/loxberry/log/plugins/{}", plugin_name),
        )
        .env(
            "LBPCONFIGDIR",
            format!("/opt/loxberry/config/plugins/{}", plugin_name),
        )
        .env("REDIRECT_STATUS", "200")
        .env("REQUEST_METHOD", &php_req.method)
        .env("QUERY_STRING", &php_req.query_string)
        .env("CONTENT_TYPE", &php_req.content_type)
        .env("CONTENT_LENGTH", php_req.body.len().to_string())
        .env("SCRIPT_FILENAME", path.to_string_lossy().to_string())
        .env("SERVER_PROTOCOL", "HTTP/1.1")
        .env("GATEWAY_INTERFACE", "CGI/1.1")
        .env("SERVER_SOFTWARE", "RustyLox");

    match cmd.output().await {
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            if !stderr.is_empty() {
                debug!("CGI stderr for plugin {}: {}", plugin_name, stderr);
            }

            let (status_code, headers, body) = parse_php_cgi_output(&out.stdout);

            let content_type = headers
                .get("content-type")
                .cloned()
                .unwrap_or_else(|| "text/html; charset=utf-8".to_string());

            let mut builder = Response::builder().status(status_code);
            builder = builder.header(header::CONTENT_TYPE, &content_type);
            for (key, value) in &headers {
                if key != "content-type" && key != "status" {
                    builder = builder.header(key.as_str(), value.as_str());
                }
            }

            builder.body(Body::from(body)).unwrap_or_else(|_| {
                error_response(StatusCode::INTERNAL_SERVER_ERROR, "Build error")
            })
        }
        Err(e) => {
            warn!(
                "Failed to execute CGI for plugin {} (is perl installed?): {}",
                plugin_name, e
            );
            error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "Perl runtime not available",
            )
        }
    }
}

/// Parse PHP CGI output: headers separated from body by double newline
fn parse_php_cgi_output(output: &[u8]) -> (StatusCode, HashMap<String, String>, Vec<u8>) {
    let mut headers = HashMap::new();
    let mut status = StatusCode::OK;

    // Look for header/body separator (\r\n\r\n or \n\n)
    let separator_pos = find_header_separator(output);

    let body = match separator_pos {
        Some((pos, sep_len)) => {
            let header_bytes = &output[..pos];
            let header_str = String::from_utf8_lossy(header_bytes);

            for line in header_str.lines() {
                if let Some((key, value)) = line.split_once(':') {
                    let key = key.trim().to_lowercase();
                    let value = value.trim().to_string();

                    if key == "status" {
                        // Parse "Status: 200 OK" or "Status: 500 Internal Server Error"
                        if let Some(code_str) = value.split_whitespace().next() {
                            if let Ok(code) = code_str.parse::<u16>() {
                                status = StatusCode::from_u16(code).unwrap_or(StatusCode::OK);
                            }
                        }
                    } else {
                        headers.insert(key, value);
                    }
                }
            }

            output[pos + sep_len..].to_vec()
        }
        None => {
            // No headers found, entire output is the body
            output.to_vec()
        }
    };

    (status, headers, body)
}

/// Find the position of the header/body separator in CGI output
fn find_header_separator(data: &[u8]) -> Option<(usize, usize)> {
    // Only look for separator if output starts with a header-like line
    // (e.g., "Content-Type:", "Status:", "X-Powered-By:")
    let prefix = String::from_utf8_lossy(&data[..data.len().min(256)]);
    let first_line = prefix.lines().next().unwrap_or("");
    if !first_line.contains(':') {
        return None;
    }

    // Look for \r\n\r\n
    for i in 0..data.len().saturating_sub(3) {
        if data[i] == b'\r' && data[i + 1] == b'\n' && data[i + 2] == b'\r' && data[i + 3] == b'\n'
        {
            return Some((i, 4));
        }
    }
    // Look for \n\n
    for i in 0..data.len().saturating_sub(1) {
        if data[i] == b'\n' && data[i + 1] == b'\n' {
            return Some((i, 2));
        }
    }
    None
}

/// Lexical path safety check - ensures the resolved path is within the base directory
fn resolve_safe_path(base: &PathBuf, path: &std::path::Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::ParentDir => {
                return None;
            }
            Component::CurDir => {}
            other => normalized.push(other),
        }
    }

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
        .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error").into_response())
}
