//! Web UI - Server-rendered interface with Askama templates and HTMX

pub mod handlers;
pub mod templates;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;
use web_api::AppState;

/// Create the web UI router
pub fn create_ui_router(state: AppState) -> Router {
    // Get static directory path
    let static_dir = state.lbhomedir.join("static");
    Router::new()
        // Dashboard
        .route("/", get(handlers::dashboard::index))
        .route("/dashboard", get(handlers::dashboard::index))
        // Miniserver management
        .route("/miniserver", get(handlers::miniserver::list))
        .route("/miniserver/monitor", get(handlers::miniserver::monitor))
        .route(
            "/miniserver/monitor/stream",
            get(handlers::miniserver::monitor_stream),
        )
        .route("/miniserver/add", get(handlers::miniserver::add_form))
        .route("/miniserver/add", post(handlers::miniserver::add_submit))
        .route("/miniserver/:id/edit", get(handlers::miniserver::edit_form))
        .route(
            "/miniserver/:id/edit",
            post(handlers::miniserver::edit_submit),
        )
        .route("/miniserver/:id/delete", post(handlers::miniserver::delete))
        .route(
            "/miniserver/:id/test",
            post(handlers::miniserver::test_connection),
        )
        // MQTT Monitor (real-time)
        .route("/mqtt/monitor", get(handlers::mqtt::monitor))
        .route("/mqtt/monitor/stream", get(handlers::mqtt::monitor_stream))
        // MQTT Configuration
        .route("/mqtt/config", get(handlers::mqtt::config))
        .route("/mqtt/config", post(handlers::mqtt::config_submit))
        // MQTT Statistics
        .route("/mqtt/stats", get(handlers::mqtt_stats::stats))
        // MQTT Subscriptions Management
        .route(
            "/mqtt/subscriptions/list",
            get(handlers::mqtt_management::list_subscriptions),
        )
        .route(
            "/mqtt/subscriptions/add",
            post(handlers::mqtt_management::add_subscription),
        )
        .route(
            "/mqtt/subscriptions/:id",
            axum::routing::delete(handlers::mqtt_management::delete_subscription),
        )
        // MQTT Conversions Management
        .route(
            "/mqtt/conversions/list",
            get(handlers::mqtt_management::list_conversions),
        )
        .route(
            "/mqtt/conversions/add",
            post(handlers::mqtt_management::add_conversion),
        )
        .route(
            "/mqtt/conversions/:id",
            axum::routing::delete(handlers::mqtt_management::delete_conversion),
        )
        // Plugin management
        .route("/plugins", get(handlers::plugins::list))
        .route("/plugins/install", get(handlers::plugins::install_form))
        .route("/plugins/install", post(handlers::plugins::install_submit))
        .route("/plugins/:md5", get(handlers::plugins::details))
        .route(
            "/plugins/:md5/uninstall",
            post(handlers::plugins::uninstall),
        )
        // Plugin web interfaces (public)
        .route(
            "/plugins/web/:name",
            get(handlers::plugin_web::serve_plugin_public_index),
        )
        .route(
            "/plugins/web/:name/*path",
            get(handlers::plugin_web::serve_plugin_public),
        )
        // Plugin web interfaces (authenticated)
        .route(
            "/admin/plugins/:name",
            get(handlers::plugin_web::serve_plugin_auth_index),
        )
        .route(
            "/admin/plugins/:name/",
            get(handlers::plugin_web::serve_plugin_auth_index),
        )
        .route(
            "/admin/plugins/:name/*path",
            get(handlers::plugin_web::serve_plugin_auth)
                .post(handlers::plugin_web::serve_plugin_auth_post),
        )
        // Email notifications
        .route("/email", get(handlers::email::index))
        // Scheduled tasks
        .route("/tasks", get(handlers::tasks::index))
        // Network diagnostics
        .route("/network", get(handlers::network::index))
        // Settings
        .route("/settings", get(handlers::settings::index))
        .route("/settings", post(handlers::settings::submit))
        // API docs
        .route("/api-docs", get(handlers::api_docs::index))
        // Log viewer
        .route("/logs", get(handlers::logs::index))
        .route("/logs/view", get(handlers::logs::view))
        // Backup management
        .route("/backup", get(handlers::backup::index))
        .route("/backup/create", post(handlers::backup::create))
        .route("/backup/:name/restore", post(handlers::backup::restore))
        .route(
            "/backup/:name",
            axum::routing::delete(handlers::backup::delete),
        )
        // Authentication
        .route("/login", get(handlers::auth::show_login))
        .route("/login", post(handlers::auth::handle_login))
        // User profile
        .route("/profile", get(handlers::profile::index))
        // System health dashboard (renamed to avoid conflict with API /health endpoint)
        .route("/system-health", get(handlers::health::index))
        // Admin: User management
        .route("/admin/users", get(handlers::auth::users))
        // Admin: API key management
        .route("/admin/api-keys", get(handlers::auth::api_keys))
        // Admin: Audit log
        .route("/admin/audit", get(handlers::auth::audit_log))
        // Admin: Security settings
        .route("/admin/security", get(handlers::security::index))
        // Admin: Data management
        .route("/admin/database", get(handlers::database::index))
        // Static files (CSS, JS, images)
        .nest_service("/static", ServeDir::new(static_dir))
        .with_state(state)
}
