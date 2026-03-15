//! Health check routes

use axum::{http::StatusCode, Json};
use serde_json::{json, Value};

/// Health check endpoint
pub async fn health_check() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "service": "loxberry-rust",
            "version": env!("CARGO_PKG_VERSION"),
        })),
    )
}
