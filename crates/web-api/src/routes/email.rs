//! Email configuration and notification API endpoints

use crate::state::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use email_manager::{
    EmailConfig, EmailConfigManager, EmailHistoryManager, EmailManager, EmailType,
};
use serde::Deserialize;
use tracing::{error, info};

/// Test email request
#[derive(Debug, Deserialize)]
pub struct TestEmailRequest {
    /// Override recipient (optional; uses config if not specified)
    pub recipient: Option<String>,
}

/// Send custom notification request
#[derive(Debug, Deserialize)]
pub struct SendEmailRequest {
    pub subject: String,
    pub message: String,
}

/// Get email configuration
///
/// GET /api/email/config
pub async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    let manager = EmailConfigManager::new(&state.lbhomedir);
    match manager.load().await {
        Ok(config) => {
            // Mask the password in the response
            let mut response_config = config.clone();
            if !response_config.smtp_pass.is_empty() {
                response_config.smtp_pass = "********".to_string();
            }
            Json(response_config).into_response()
        }
        Err(e) => {
            error!("Failed to load email config: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("{}", e) })),
            )
                .into_response()
        }
    }
}

/// Update email configuration
///
/// PUT /api/email/config
pub async fn update_config(
    State(state): State<AppState>,
    Json(mut new_config): Json<EmailConfig>,
) -> impl IntoResponse {
    let manager = EmailConfigManager::new(&state.lbhomedir);

    // If password is masked, keep the existing password
    if new_config.smtp_pass == "********" {
        if let Ok(existing) = manager.load().await {
            new_config.smtp_pass = existing.smtp_pass;
        }
    }

    match manager.save(&new_config).await {
        Ok(_) => {
            info!("Email configuration updated");
            Json(serde_json::json!({ "success": true, "message": "Email configuration saved" }))
                .into_response()
        }
        Err(e) => {
            error!("Failed to save email config: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("{}", e) })),
            )
                .into_response()
        }
    }
}

/// Send a test email
///
/// POST /api/email/test
pub async fn send_test(
    State(state): State<AppState>,
    Json(req): Json<TestEmailRequest>,
) -> impl IntoResponse {
    let config_manager = EmailConfigManager::new(&state.lbhomedir);
    let config = match config_manager.load().await {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to load config: {}", e) })),
            )
                .into_response();
        }
    };

    let recipient = req.recipient.unwrap_or_else(|| {
        config
            .notification_addresses
            .first()
            .cloned()
            .unwrap_or_default()
    });

    if recipient.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "No recipient configured" })),
        )
            .into_response();
    }

    let email_manager = EmailManager::new(config, &state.version).with_history(&state.lbhomedir);
    let result = email_manager.send_test(&recipient).await;

    if result.success {
        Json(serde_json::json!({
            "success": true,
            "message": format!("Test email sent to {}", result.recipient)
        }))
        .into_response()
    } else {
        (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "success": false,
                "error": result.error.unwrap_or_else(|| "Unknown error".to_string())
            })),
        )
            .into_response()
    }
}

/// Send a custom notification email to all configured recipients
///
/// POST /api/email/send
pub async fn send_notification(
    State(state): State<AppState>,
    Json(req): Json<SendEmailRequest>,
) -> impl IntoResponse {
    let config_manager = EmailConfigManager::new(&state.lbhomedir);
    let config = match config_manager.load().await {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to load config: {}", e) })),
            )
                .into_response();
        }
    };

    if !config.enabled {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Email notifications are disabled" })),
        )
            .into_response();
    }

    let email_manager = EmailManager::new(config, &state.version).with_history(&state.lbhomedir);
    let email_type = EmailType::Custom {
        subject: req.subject,
        body: req.message,
    };
    let results = email_manager.send_notification(&email_type).await;

    let all_success = results.iter().all(|r| r.success);
    let failed: Vec<_> = results
        .iter()
        .filter(|r| !r.success)
        .map(|r| {
            serde_json::json!({
                "recipient": r.recipient,
                "error": r.error
            })
        })
        .collect();

    Json(serde_json::json!({
        "success": all_success,
        "sent_count": results.len(),
        "failed": failed
    }))
    .into_response()
}

/// Get email send history
///
/// GET /api/email/history
pub async fn get_history(State(state): State<AppState>) -> impl IntoResponse {
    let history_manager = EmailHistoryManager::new(&state.lbhomedir);
    let history = history_manager.recent(50).await;
    Json(history).into_response()
}
