//! API documentation handler

use crate::templates::{ApiDocsTemplate, ApiEndpoint, ApiGroup};
use askama::Template;
use axum::{extract::State, response::Html};
use web_api::AppState;

pub async fn index(State(state): State<AppState>) -> Html<String> {
    let groups = vec![
        ApiGroup {
            name: "Health & Metrics".to_string(),
            endpoints: vec![
                ep("GET", "/health", "Service health check"),
                ep("GET", "/api/health", "Service health check (API path)"),
                ep(
                    "GET",
                    "/api/health/detail",
                    "Detailed health check with per-component status (config, MQTT, Miniserver, disk, CPU, memory)",
                ),
                ep("GET", "/metrics", "Prometheus metrics (text/plain)"),
                ep("GET", "/api/system/metrics", "System metrics as JSON (CPU, memory, disk, uptime)"),
            ],
        },
        ApiGroup {
            name: "System".to_string(),
            endpoints: vec![
                ep(
                    "GET",
                    "/api/system/status",
                    "System status, version, MQTT state",
                ),
                ep("GET", "/api/system/log-level", "Get current log level"),
                ep(
                    "PUT",
                    "/api/system/log-level",
                    "Set log level (body: {log_level: \"debug\"})",
                ),
                ep("GET", "/api/system/update/check", "Check for a newer GitHub release"),
                ep("POST", "/api/system/update/apply", "Apply a pending system update"),
            ],
        },
        ApiGroup {
            name: "Configuration".to_string(),
            endpoints: vec![
                ep("GET", "/api/config/general", "Get full general.json config"),
                ep(
                    "PUT",
                    "/api/config/general",
                    "Replace full general.json config",
                ),
            ],
        },
        ApiGroup {
            name: "Miniserver".to_string(),
            endpoints: vec![
                ep("GET", "/api/miniserver", "List all configured Miniservers"),
                ep("GET", "/api/miniserver/:id", "Get single Miniserver config"),
                ep(
                    "GET",
                    "/api/miniserver/:id/status",
                    "Check connection status",
                ),
                ep(
                    "POST",
                    "/api/miniserver/:id/send",
                    "Send HTTP command to Miniserver",
                ),
                ep(
                    "POST",
                    "/api/miniserver/:id/get",
                    "Get values from Miniserver",
                ),
            ],
        },
        ApiGroup {
            name: "MQTT Gateway".to_string(),
            endpoints: vec![
                ep("GET", "/api/mqtt/status", "Gateway connection status"),
                ep("GET", "/api/mqtt/relayed-topics", "List relayed topic cache entries"),
                ep("POST", "/api/mqtt/topic-settings", "Update per-topic relay settings"),
                ep("POST", "/api/mqtt/topic-delete", "Remove a topic from the relay cache"),
                ep("POST", "/api/mqtt/relay-cache/clear", "Clear the entire relay cache"),
                ep("GET", "/api/mqtt/finder", "MQTT Finder — recent messages per topic"),
                ep(
                    "POST",
                    "/api/mqtt/subscriptions/reload",
                    "Reload subscriptions from file",
                ),
                ep(
                    "POST",
                    "/api/mqtt/transformers/reload",
                    "Reload transformers from file",
                ),
            ],
        },
        ApiGroup {
            name: "MQTT Statistics".to_string(),
            endpoints: vec![
                ep("GET", "/api/mqtt/stats", "Gateway message statistics"),
                ep("GET", "/api/mqtt/rejected", "List rejected/filtered MQTT params"),
                ep("POST", "/api/mqtt/stats/reset", "Reset statistics counters"),
            ],
        },
        ApiGroup {
            name: "Plugins".to_string(),
            endpoints: vec![
                ep("GET", "/api/plugins", "List installed plugins"),
                ep("GET", "/api/plugins/:md5", "Get plugin details"),
                ep(
                    "POST",
                    "/api/plugins/install",
                    "Install plugin from ZIP (multipart)",
                ),
                ep("DELETE", "/api/plugins/:md5", "Uninstall plugin"),
                ep(
                    "POST",
                    "/api/plugins/:md5/upgrade",
                    "Upgrade plugin from ZIP",
                ),
            ],
        },
        ApiGroup {
            name: "Plugin Daemons".to_string(),
            endpoints: vec![
                ep("POST", "/api/plugins/:folder/daemon/start", "Start plugin daemon"),
                ep("POST", "/api/plugins/:folder/daemon/stop", "Stop plugin daemon"),
                ep("POST", "/api/plugins/:folder/daemon/restart", "Restart plugin daemon"),
                ep("GET", "/api/plugins/:folder/daemon/status", "Get daemon status"),
                ep("GET", "/api/plugins/:folder/daemon/logs", "Stream daemon log tail"),
            ],
        },
        ApiGroup {
            name: "Backup".to_string(),
            endpoints: vec![
                ep("GET", "/api/backup", "List all backups"),
                ep(
                    "POST",
                    "/api/backup/create",
                    "Create backup (?include_plugins=true)",
                ),
                ep("GET", "/api/backup/:name/download", "Download backup file"),
                ep("POST", "/api/backup/:name/restore", "Restore from backup"),
                ep("DELETE", "/api/backup/:name", "Delete a backup"),
                ep("GET", "/api/backup/schedule", "Get backup schedule config"),
                ep(
                    "PUT",
                    "/api/backup/schedule",
                    "Update backup schedule config",
                ),
            ],
        },
        ApiGroup {
            name: "Scheduled Tasks".to_string(),
            endpoints: vec![
                ep("GET", "/api/tasks", "List all scheduled tasks"),
                ep("POST", "/api/tasks", "Create a new scheduled task"),
                ep("GET", "/api/tasks/history", "Get recent task execution history"),
                ep("GET", "/api/tasks/:id", "Get a single task by ID"),
                ep("PUT", "/api/tasks/:id", "Update a scheduled task"),
                ep("DELETE", "/api/tasks/:id", "Delete a scheduled task"),
                ep("POST", "/api/tasks/:id/run", "Manually trigger a task immediately"),
            ],
        },
        ApiGroup {
            name: "Network Diagnostics".to_string(),
            endpoints: vec![
                ep("POST", "/api/network/ping", "Ping a host (body: {host, count?})"),
                ep("GET", "/api/network/interfaces", "List network interfaces with IP/MAC/status"),
                ep(
                    "POST",
                    "/api/network/test/connection",
                    "Test TCP connectivity (body: {host, port})",
                ),
                ep(
                    "POST",
                    "/api/network/test/miniserver",
                    "Test Miniserver TCP connectivity (body: {id})",
                ),
                ep("POST", "/api/network/test/mqtt", "Test MQTT broker TCP connectivity"),
            ],
        },
        ApiGroup {
            name: "Email".to_string(),
            endpoints: vec![
                ep("GET", "/api/email/config", "Get SMTP configuration"),
                ep("PUT", "/api/email/config", "Update SMTP configuration"),
                ep("POST", "/api/email/test", "Send a test email"),
                ep("POST", "/api/email/send", "Send a notification email"),
                ep("GET", "/api/email/history", "Get recent email send history"),
            ],
        },
        ApiGroup {
            name: "Weather".to_string(),
            endpoints: vec![
                ep("GET", "/api/weather/status", "Weather service status"),
                ep("GET", "/api/weather/current", "Current weather data"),
                ep("GET", "/api/weather/forecast", "Daily forecast"),
                ep("GET", "/api/weather/hourly", "Hourly forecast"),
                ep("GET", "/api/weather/all", "All weather data combined"),
                ep("GET", "/api/weather/config", "Get weather service configuration"),
                ep("PUT", "/api/weather/config", "Update weather service configuration"),
                ep("POST", "/api/weather/refresh", "Force a weather data refresh"),
                ep("GET", "/forecast", "Loxone Cloud Emulator forecast endpoint"),
            ],
        },
        ApiGroup {
            name: "Authentication & Users".to_string(),
            endpoints: vec![
                ep("POST", "/api/auth/login", "Login — returns JWT cookie"),
                ep("POST", "/api/auth/logout", "Logout — clears JWT cookie"),
                ep("GET", "/api/auth/me", "Get current authenticated user"),
                ep("GET", "/api/auth/keys", "List API keys for current user"),
                ep("POST", "/api/auth/keys", "Create a new API key"),
                ep("DELETE", "/api/auth/keys/:id", "Revoke an API key"),
                ep("GET", "/api/auth/audit", "Get audit log entries"),
                ep("GET", "/api/users", "List all users (admin)"),
                ep("POST", "/api/users", "Create a new user (admin)"),
                ep("DELETE", "/api/users/:id", "Delete a user (admin)"),
                ep("PUT", "/api/users/:id/password", "Change a user's password"),
            ],
        },
        ApiGroup {
            name: "Virtual Inputs".to_string(),
            endpoints: vec![
                ep(
                    "GET",
                    "/dev/sps/io/:name/:value",
                    "Receive value from Miniserver Virtual HTTP Output",
                ),
                ep(
                    "GET",
                    "/dev/sps/io/:name",
                    "Receive named signal from Miniserver (no value)",
                ),
            ],
        },
    ];

    let template = ApiDocsTemplate {
        groups,
        version: state.version.clone(),
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

fn ep(method: &str, path: &str, description: &str) -> ApiEndpoint {
    ApiEndpoint {
        method: method.to_string(),
        path: path.to_string(),
        description: description.to_string(),
    }
}
