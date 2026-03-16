//! Web API - REST API for LoxBerry management

pub mod routes;
pub mod state;

pub use state::AppState;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

/// Create the Axum router with all routes
pub fn create_router(state: AppState) -> Router {
    // Rate limit: 60 requests/minute per IP (replenish 1 req/s, burst 10)
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(10)
            .finish()
            .expect("Invalid rate limit config"),
    );

    Router::new()
        // Health check
        .route("/health", get(routes::health::health_check))
        .route("/api/health", get(routes::health::health_check))
        .route("/api/health/detail", get(routes::metrics::detailed_health))
        // Metrics
        .route("/metrics", get(routes::metrics::prometheus_metrics))
        .route("/api/system/metrics", get(routes::metrics::system_metrics))
        // Configuration routes
        .route("/api/config/general", get(routes::config::get_general))
        .route("/api/config/general", put(routes::config::update_general))
        // Miniserver routes
        .route("/api/miniserver", get(routes::miniserver::list_miniservers))
        .route(
            "/api/miniserver/:id",
            get(routes::miniserver::get_miniserver),
        )
        .route(
            "/api/miniserver/:id/send",
            post(routes::miniserver::send_command),
        )
        .route(
            "/api/miniserver/:id/get",
            post(routes::miniserver::get_values),
        )
        .route(
            "/api/miniserver/:id/status",
            get(routes::miniserver::check_status),
        )
        // Plugin routes
        .route("/api/plugins", get(routes::plugins::list_plugins))
        .route("/api/plugins/:md5", get(routes::plugins::get_plugin))
        .route(
            "/api/plugins/install",
            post(routes::plugins::install_plugin),
        )
        .route(
            "/api/plugins/:md5",
            delete(routes::plugins::uninstall_plugin),
        )
        .route(
            "/api/plugins/:md5/upgrade",
            post(routes::plugins::upgrade_plugin),
        )
        // Plugin daemon routes (by folder name)
        .route(
            "/api/plugins/:folder/daemon/start",
            post(routes::daemon::start_daemon),
        )
        .route(
            "/api/plugins/:folder/daemon/stop",
            post(routes::daemon::stop_daemon),
        )
        .route(
            "/api/plugins/:folder/daemon/restart",
            post(routes::daemon::restart_daemon),
        )
        .route(
            "/api/plugins/:folder/daemon/status",
            get(routes::daemon::get_daemon_status),
        )
        .route(
            "/api/plugins/:folder/daemon/logs",
            get(routes::daemon::get_daemon_logs),
        )
        // MQTT Gateway routes
        .route("/api/mqtt/status", get(routes::mqtt::get_status))
        .route(
            "/api/mqtt/subscriptions/reload",
            post(routes::mqtt::reload_subscriptions),
        )
        .route(
            "/api/mqtt/transformers/reload",
            post(routes::mqtt::reload_transformers),
        )
        // System routes
        .route("/api/system/status", get(routes::system::system_status))
        .route("/api/system/log-level", get(routes::system::get_log_level))
        .route("/api/system/log-level", put(routes::system::set_log_level))
        // Email routes
        .route("/api/email/config", get(routes::email::get_config))
        .route("/api/email/config", put(routes::email::update_config))
        .route("/api/email/test", post(routes::email::send_test))
        .route("/api/email/send", post(routes::email::send_notification))
        // Scheduled task routes
        .route("/api/tasks", get(routes::tasks::list_tasks))
        .route("/api/tasks", post(routes::tasks::create_task))
        .route("/api/tasks/history", get(routes::tasks::get_history))
        .route("/api/tasks/:id", get(routes::tasks::get_task))
        .route("/api/tasks/:id", put(routes::tasks::update_task))
        .route("/api/tasks/:id", delete(routes::tasks::delete_task))
        .route("/api/tasks/:id/run", post(routes::tasks::run_task))
        // Network diagnostics routes
        .route("/api/network/ping", post(routes::network::ping_host))
        .route(
            "/api/network/interfaces",
            get(routes::network::list_interfaces),
        )
        .route(
            "/api/network/test/connection",
            post(routes::network::test_connection),
        )
        .route(
            "/api/network/test/miniserver",
            post(routes::network::test_miniserver),
        )
        .route("/api/network/test/mqtt", post(routes::network::test_mqtt))
        // Backup routes
        .route("/api/backup", get(routes::backup::list_backups))
        .route("/api/backup/create", post(routes::backup::create_backup))
        .route("/api/backup/schedule", get(routes::backup::get_schedule))
        .route("/api/backup/schedule", put(routes::backup::update_schedule))
        .route(
            "/api/backup/:name/download",
            get(routes::backup::download_backup),
        )
        .route(
            "/api/backup/:name/restore",
            post(routes::backup::restore_backup),
        )
        .route("/api/backup/:name", delete(routes::backup::delete_backup))
        .with_state(state)
        // Middleware (innermost first)
        .layer(GovernorLayer {
            config: governor_conf,
        })
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
