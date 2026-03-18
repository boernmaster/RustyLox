//! Metrics and system info API endpoints

use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use loxberry_metrics::{
    collector::MetricsCollector,
    health::{ComponentStatus, HealthCheck, HealthMetrics},
};
use tracing::error;

/// Prometheus metrics endpoint
///
/// GET /metrics
pub async fn prometheus_metrics(State(_state): State<AppState>) -> Response {
    let mut collector = MetricsCollector::with_default_counters();
    let system = collector.collect_system();
    let app = collector.collect_app();
    let uptime = collector.daemon_uptime_seconds();

    let output = loxberry_metrics::PrometheusExporter::export(&system, &app, uptime);

    (
        StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4; charset=utf-8")],
        output,
    )
        .into_response()
}

/// System metrics as JSON
///
/// GET /api/system/metrics
pub async fn system_metrics(State(_state): State<AppState>) -> impl IntoResponse {
    let mut collector = MetricsCollector::with_default_counters();
    let metrics = collector.collect_system();
    Json(metrics)
}

/// Enhanced health check with component status
///
/// GET /api/health/detail
pub async fn detailed_health(State(state): State<AppState>) -> impl IntoResponse {
    let mut components = Vec::new();

    // Config check
    {
        let config = state.config.read().await;
        if config.base.version.is_empty() {
            components.push(ComponentStatus::degraded("config", "Version not set"));
        } else {
            components.push(ComponentStatus::ok("config"));
        }
    }

    // MQTT check
    if let Some(ref gateway) = state.mqtt_gateway {
        if gateway.status().connected {
            components.push(ComponentStatus::ok("mqtt_broker"));
        } else {
            components.push(ComponentStatus::degraded(
                "mqtt_broker",
                "MQTT broker not connected",
            ));
        }
    } else {
        components.push(ComponentStatus::degraded(
            "mqtt_broker",
            "MQTT gateway not configured",
        ));
    }

    // Miniserver checks
    {
        let config = state.config.read().await;
        if config.miniserver.is_empty() {
            components.push(ComponentStatus::degraded(
                "miniserver",
                "No miniserver configured",
            ));
        } else {
            // Check actual connectivity for first miniserver
            let mut all_healthy = true;
            let mut error_msg = String::new();

            for (id, _ms) in &config.miniserver {
                // Parse ID to u8
                let id_num = match id.parse::<u8>() {
                    Ok(n) => n,
                    Err(_) => {
                        all_healthy = false;
                        error_msg = format!("MS {}: Invalid ID", id);
                        break;
                    }
                };

                if let Ok(client) = state.get_miniserver_client(id_num).await {
                    // Try simple call to check connectivity
                    match client.http().call("/dev/lan/txp").await {
                        Ok(_) => continue,
                        Err(e) => {
                            all_healthy = false;
                            error_msg = format!("MS {}: {}", id, e);
                            break;
                        }
                    }
                } else {
                    all_healthy = false;
                    error_msg = format!("MS {}: Failed to create client", id);
                    break;
                }
            }

            if all_healthy {
                components.push(ComponentStatus::ok("miniserver"));
            } else {
                components.push(ComponentStatus::degraded("miniserver", error_msg));
            }
        }
    }

    // Disk space check
    let mut sys_collector = MetricsCollector::with_default_counters();
    let sys_metrics = sys_collector.collect_system();

    if sys_metrics.disk_usage_percent > 95.0 {
        components.push(ComponentStatus::unhealthy(
            "disk_space",
            format!(
                "Disk usage critical: {:.1}%",
                sys_metrics.disk_usage_percent
            ),
        ));
    } else if sys_metrics.disk_usage_percent > 85.0 {
        components.push(ComponentStatus::degraded(
            "disk_space",
            format!("Disk usage high: {:.1}%", sys_metrics.disk_usage_percent),
        ));
    } else {
        components.push(ComponentStatus::ok("disk_space"));
    }

    // Memory check
    if sys_metrics.memory_usage_percent > 95.0 {
        components.push(ComponentStatus::unhealthy(
            "memory",
            format!(
                "Memory usage critical: {:.1}%",
                sys_metrics.memory_usage_percent
            ),
        ));
    } else if sys_metrics.memory_usage_percent > 85.0 {
        components.push(ComponentStatus::degraded(
            "memory",
            format!(
                "Memory usage high: {:.1}%",
                sys_metrics.memory_usage_percent
            ),
        ));
    } else {
        components.push(ComponentStatus::ok("memory"));
    }

    // Plugin count
    let plugin_count = {
        let installer = plugin_manager::PluginInstaller::new(&state.lbhomedir);
        match installer.list().await {
            Ok(plugins) => plugins.len(),
            Err(e) => {
                error!("Failed to list plugins for health check: {}", e);
                0
            }
        }
    };

    let miniserver_count = {
        let config = state.config.read().await;
        config.miniserver.len()
    };

    let mqtt_connected = state
        .mqtt_gateway
        .as_ref()
        .map(|g| g.status().connected)
        .unwrap_or(false);

    let health_metrics = HealthMetrics {
        cpu_usage_percent: sys_metrics.cpu_usage_percent,
        memory_usage_percent: sys_metrics.memory_usage_percent,
        disk_usage_percent: sys_metrics.disk_usage_percent,
        mqtt_connected,
        active_plugins: plugin_count,
        miniserver_count,
    };

    let health = HealthCheck::build(
        state.version.clone(),
        sys_metrics.uptime_seconds,
        components,
        health_metrics,
    );

    let status_code = match &health.status {
        loxberry_metrics::HealthStatus::Healthy => StatusCode::OK,
        loxberry_metrics::HealthStatus::Degraded => StatusCode::OK,
        loxberry_metrics::HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status_code, Json(health)).into_response()
}
