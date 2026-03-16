//! Health check routes

use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::AppState;

/// Health check endpoint
pub async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "service": "loxberry-rust",
            "version": state.version,
        })),
    )
}
