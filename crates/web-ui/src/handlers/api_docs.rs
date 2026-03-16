//! API documentation handler

use crate::templates::{ApiDocsTemplate, ApiEndpoint, ApiGroup};
use askama::Template;
use axum::{extract::State, response::Html};
use web_api::AppState;

pub async fn index(State(state): State<AppState>) -> Html<String> {
    let groups = vec![
        ApiGroup {
            name: "Health".to_string(),
            endpoints: vec![
                ep("GET", "/health", "Service health check"),
                ep("GET", "/api/health", "Service health check (API path)"),
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
