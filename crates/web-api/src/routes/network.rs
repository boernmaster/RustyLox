//! Network diagnostics API endpoints

use crate::state::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Ping request
#[derive(Debug, Deserialize)]
pub struct PingRequest {
    /// Host to ping
    pub host: String,
    /// Number of pings (max 5)
    pub count: Option<u8>,
}

/// Ping result
#[derive(Debug, Serialize)]
pub struct PingResult {
    pub host: String,
    pub reachable: bool,
    pub latency_ms: Option<f64>,
    pub error: Option<String>,
}

/// Network interface info
#[derive(Debug, Serialize)]
pub struct NetworkInterface {
    pub name: String,
    pub ip_addresses: Vec<String>,
    pub mac_address: Option<String>,
    pub is_up: bool,
}

/// Connection test request
#[derive(Debug, Deserialize)]
pub struct ConnectionTestRequest {
    pub host: String,
    pub port: u16,
}

/// Connection test result
#[derive(Debug, Serialize)]
pub struct ConnectionTestResult {
    pub host: String,
    pub port: u16,
    pub reachable: bool,
    pub latency_ms: Option<f64>,
    pub error: Option<String>,
}

/// Ping a host
///
/// POST /api/network/ping
pub async fn ping_host(
    State(_state): State<AppState>,
    Json(req): Json<PingRequest>,
) -> impl IntoResponse {
    // Basic host validation
    if req.host.is_empty() || req.host.len() > 253 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid host" })),
        )
            .into_response();
    }

    // Sanitize host - only allow alphanumeric, dots, hyphens, colons (IPv6)
    if !req
        .host
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == ':')
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid host characters" })),
        )
            .into_response();
    }

    let count = req.count.unwrap_or(3).min(5);
    let result = ping_host_impl(&req.host, count).await;
    Json(result).into_response()
}

/// Get network interfaces
///
/// GET /api/network/interfaces
pub async fn list_interfaces(State(_state): State<AppState>) -> impl IntoResponse {
    let interfaces = get_network_interfaces().await;
    Json(serde_json::to_value(interfaces).unwrap_or_default())
}

/// Test TCP connection to host:port
///
/// POST /api/network/test/connection
pub async fn test_connection(
    State(_state): State<AppState>,
    Json(req): Json<ConnectionTestRequest>,
) -> impl IntoResponse {
    // Validate
    if req.host.is_empty() || req.port == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid host or port" })),
        )
            .into_response();
    }

    let result = test_tcp_connection(&req.host, req.port).await;
    Json(result).into_response()
}

/// Test Miniserver connectivity
///
/// POST /api/network/test/miniserver
pub async fn test_miniserver(
    State(state): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let id = req.get("id").and_then(|v| v.as_str()).unwrap_or("1");

    let config = state.config.read().await;
    let ms = match config.miniserver.get(id) {
        Some(ms) => ms.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": format!("Miniserver {} not found", id) })),
            )
                .into_response();
        }
    };
    drop(config);

    let port: u16 = ms.port.parse().unwrap_or(80);
    let result = test_tcp_connection(&ms.ipaddress, port).await;
    Json(result).into_response()
}

/// Test MQTT broker connectivity
///
/// POST /api/network/test/mqtt
pub async fn test_mqtt(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config.read().await;
    let host = config.mqtt.brokerhost.clone();
    let port: u16 = config.mqtt.brokerport.parse().unwrap_or(1883);
    drop(config);

    if host.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "MQTT broker not configured" })),
        )
            .into_response();
    }

    let result = test_tcp_connection(&host, port).await;
    Json(result).into_response()
}

/// Perform a ping using system ping command
async fn ping_host_impl(host: &str, count: u8) -> PingResult {
    let start = Instant::now();

    let output = tokio::process::Command::new("ping")
        .arg("-c")
        .arg(count.to_string())
        .arg("-W")
        .arg("2") // 2 second timeout per ping
        .arg(host)
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            let elapsed = start.elapsed().as_secs_f64() * 1000.0 / count as f64;
            // Try to parse actual latency from ping output
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let latency = parse_ping_latency(&stdout).unwrap_or(elapsed);

            PingResult {
                host: host.to_string(),
                reachable: true,
                latency_ms: Some(latency),
                error: None,
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            PingResult {
                host: host.to_string(),
                reachable: false,
                latency_ms: None,
                error: Some(if stderr.is_empty() {
                    "Host unreachable".to_string()
                } else {
                    stderr
                }),
            }
        }
        Err(e) => PingResult {
            host: host.to_string(),
            reachable: false,
            latency_ms: None,
            error: Some(format!("ping command failed: {}", e)),
        },
    }
}

/// Parse average latency from ping output (Linux format)
fn parse_ping_latency(output: &str) -> Option<f64> {
    // Look for "rtt min/avg/max/mdev = X.XXX/X.XXX/X.XXX/X.XXX ms"
    for line in output.lines() {
        if line.contains("rtt") && line.contains("avg") {
            if let Some(part) = line.split('/').nth(1) {
                if let Ok(val) = part.trim().parse::<f64>() {
                    return Some(val);
                }
            }
        }
    }
    None
}

/// Test TCP connectivity to host:port
async fn test_tcp_connection(host: &str, port: u16) -> ConnectionTestResult {
    use tokio::net::TcpStream;
    use tokio::time::timeout;

    let start = Instant::now();
    let addr = format!("{}:{}", host, port);

    let result = timeout(Duration::from_secs(5), TcpStream::connect(&addr)).await;

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

    match result {
        Ok(Ok(_stream)) => ConnectionTestResult {
            host: host.to_string(),
            port,
            reachable: true,
            latency_ms: Some(elapsed_ms),
            error: None,
        },
        Ok(Err(e)) => ConnectionTestResult {
            host: host.to_string(),
            port,
            reachable: false,
            latency_ms: None,
            error: Some(format!("Connection failed: {}", e)),
        },
        Err(_) => ConnectionTestResult {
            host: host.to_string(),
            port,
            reachable: false,
            latency_ms: None,
            error: Some("Connection timed out".to_string()),
        },
    }
}

/// Get network interface information
async fn get_network_interfaces() -> Vec<NetworkInterface> {
    use sysinfo::Networks;

    let networks = Networks::new_with_refreshed_list();
    networks
        .iter()
        .map(|(name, data)| NetworkInterface {
            name: name.clone(),
            ip_addresses: data
                .ip_networks()
                .iter()
                .map(|ip| ip.addr.to_string())
                .collect(),
            mac_address: Some(data.mac_address().to_string()),
            is_up: !data.ip_networks().is_empty(),
        })
        .collect()
}
