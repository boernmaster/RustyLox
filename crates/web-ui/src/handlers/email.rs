//! Email configuration UI handler

use crate::templates::EmailTemplate;
use askama::Template;
use axum::{extract::State, response::Html};
use email_manager::EmailConfigManager;
use web_api::AppState;

/// Email configuration page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let manager = EmailConfigManager::new(&state.lbhomedir);
    let config = manager.load().await.unwrap_or_default();

    let mut masked_pass = config.smtp_pass.clone();
    if !masked_pass.is_empty() {
        masked_pass = "********".to_string();
    }

    let lang = state.config.read().await.base.lang.clone();
    let template = EmailTemplate {
        smtp_host: config.smtp_host,
        smtp_port: config.smtp_port,
        smtp_user: config.smtp_user,
        smtp_pass: masked_pass,
        smtp_tls: config.smtp_tls,
        from_address: config.from_address,
        from_name: config.from_name,
        notification_addresses: config.notification_addresses.join("\n"),
        enabled: config.enabled,
        version: state.version.clone(),
        lang,
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}
