//! Web API - REST API for LoxBerry management

pub mod routes;
pub mod state;

pub use state::AppState;

use axum::{
    routing::{get, post, put},
    Router,
};
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};

/// Create the Axum router with all routes
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/", get(routes::health::health_check))
        .route("/health", get(routes::health::health_check))
        // Configuration routes
        .route("/api/config/general", get(routes::config::get_general))
        .route("/api/config/general", put(routes::config::update_general))
        // Miniserver routes
        .route("/api/miniserver", get(routes::miniserver::list_miniservers))
        .route("/api/miniserver/:id", get(routes::miniserver::get_miniserver))
        .route("/api/miniserver/:id/send", post(routes::miniserver::send_command))
        .route("/api/miniserver/:id/get", post(routes::miniserver::get_values))
        .route("/api/miniserver/:id/status", get(routes::miniserver::check_status))
        // System routes
        .route("/api/system/status", get(routes::system::system_status))
        .with_state(state)
        // Middleware
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
