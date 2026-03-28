//! Web API - REST API for LoxBerry management

pub mod middleware;
pub mod routes;
pub mod state;
pub mod weather;

pub use state::{AppState, MiniserverEvent};

use axum::{
    middleware as axum_middleware,
    routing::{delete, get, post, put},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

/// Create the Axum router with all routes
pub fn create_router(state: AppState) -> Router {
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
            "/api/mqtt/relayed-topics",
            get(routes::mqtt::get_relayed_topics),
        )
        .route(
            "/api/mqtt/topic-settings",
            post(routes::mqtt::update_topic_setting),
        )
        .route(
            "/api/mqtt/topic-delete",
            post(routes::mqtt::delete_topic_cache),
        )
        .route(
            "/api/mqtt/relay-cache/clear",
            post(routes::mqtt::clear_relay_cache),
        )
        .route(
            "/api/mqtt/subscriptions/reload",
            post(routes::mqtt::reload_subscriptions),
        )
        .route(
            "/api/mqtt/transformers/reload",
            post(routes::mqtt::reload_transformers),
        )
        // MQTT Statistics routes
        .route("/api/mqtt/stats", get(routes::mqtt_stats::get_stats))
        .route(
            "/api/mqtt/rejected",
            get(routes::mqtt_stats::get_rejected_params),
        )
        .route(
            "/api/mqtt/stats/reset",
            post(routes::mqtt_stats::reset_stats),
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
        .route("/api/email/history", get(routes::email::get_history))
        // System update routes
        .route(
            "/api/system/update/check",
            get(routes::system_update::check_update),
        )
        .route(
            "/api/system/update/apply",
            post(routes::system_update::apply_update),
        )
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
        // Weather routes
        .route("/api/weather/status", get(routes::weather::status))
        .route("/api/weather/current", get(routes::weather::current))
        .route("/api/weather/forecast", get(routes::weather::forecast))
        .route("/api/weather/hourly", get(routes::weather::hourly))
        .route("/api/weather/all", get(routes::weather::all))
        .route("/api/weather/config", get(routes::weather::get_config))
        .route("/api/weather/config", put(routes::weather::update_config))
        .route("/api/weather/refresh", post(routes::weather::refresh))
        // Virtual Input endpoint – receives data from Miniserver Virtual HTTP Outputs
        // The Miniserver calls: http://<RustyLox>:8080/dev/sps/io/<name>/<value>
        .route(
            "/dev/sps/io/:name/:value",
            get(routes::virtual_input::receive_value),
        )
        .route(
            "/dev/sps/io/:name",
            get(routes::virtual_input::receive_name_only),
        )
        // Loxone Cloud Emulator (served on main port; daemon also starts port 6066)
        .route("/forecast/", get(routes::weather::loxone_forecast))
        .route("/forecast", get(routes::weather::loxone_forecast))
        // Auth routes
        .route("/api/auth/login", post(routes::auth::login))
        .route("/api/auth/logout", post(routes::auth::logout))
        .route("/api/auth/me", get(routes::auth::me))
        .route("/api/auth/keys", get(routes::auth::list_api_keys))
        .route("/api/auth/keys", post(routes::auth::create_api_key))
        .route("/api/auth/keys/:id", delete(routes::auth::delete_api_key))
        .route("/api/auth/audit", get(routes::auth::get_audit_log))
        // User management routes
        .route("/api/users", get(routes::auth::list_users))
        .route("/api/users", post(routes::auth::create_user))
        .route("/api/users/:id", delete(routes::auth::delete_user))
        .route(
            "/api/users/:id/password",
            put(routes::auth::change_password),
        )
        .with_state(state)
        // Middleware (innermost first)
        .layer(axum_middleware::from_fn(
            middleware::security_headers::add_security_headers,
        ))
        // Note: Rate limiting disabled due to issues with GovernorLayer in Docker
        // TODO: Re-enable with proper IP extraction once fixed
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
