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
        .route("/miniserver/add", get(handlers::miniserver::add_form))
        .route("/miniserver/add", post(handlers::miniserver::add_submit))
        .route("/miniserver/:id/edit", get(handlers::miniserver::edit_form))
        .route("/miniserver/:id/edit", post(handlers::miniserver::edit_submit))
        .route("/miniserver/:id/delete", post(handlers::miniserver::delete))
        .route("/miniserver/:id/test", post(handlers::miniserver::test_connection))

        // MQTT Monitor (real-time)
        .route("/mqtt/monitor", get(handlers::mqtt::monitor))
        .route("/mqtt/monitor/stream", get(handlers::mqtt::monitor_stream))

        // MQTT Configuration
        .route("/mqtt/config", get(handlers::mqtt::config))
        .route("/mqtt/config", post(handlers::mqtt::config_submit))
        .route("/mqtt/subscriptions", get(handlers::mqtt::subscriptions))
        .route("/mqtt/subscription/add", post(handlers::mqtt::add_subscription))
        .route("/mqtt/subscription/:id/delete", post(handlers::mqtt::delete_subscription))

        // Plugin management
        .route("/plugins", get(handlers::plugins::list))
        .route("/plugins/install", get(handlers::plugins::install_form))
        .route("/plugins/install", post(handlers::plugins::install_submit))
        .route("/plugins/:md5", get(handlers::plugins::details))
        .route("/plugins/:md5/uninstall", post(handlers::plugins::uninstall))

        // Settings
        .route("/settings", get(handlers::settings::index))
        .route("/settings", post(handlers::settings::submit))

        // Static files (CSS, JS, images)
        .nest_service("/static", ServeDir::new(static_dir))

        .with_state(state)
}
