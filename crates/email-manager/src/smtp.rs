//! SMTP email client using lettre

use crate::config::EmailConfig;
use crate::templates::{EmailTemplate, EmailType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Maximum number of email history entries to keep
const MAX_EMAIL_HISTORY: usize = 100;

/// Result of an email send attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResult {
    pub success: bool,
    pub recipient: String,
    pub error: Option<String>,
    pub sent_at: DateTime<Utc>,
}

/// Email history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailHistoryEntry {
    pub subject: String,
    pub recipients: Vec<String>,
    pub results: Vec<SendResult>,
    pub sent_at: DateTime<Utc>,
}

/// Manages loading and saving email send history
pub struct EmailHistoryManager {
    history_path: PathBuf,
}

impl EmailHistoryManager {
    pub fn new(lbhomedir: &Path) -> Self {
        Self {
            history_path: lbhomedir.join("data/system/email_history.json"),
        }
    }

    /// Load history from disk
    pub async fn load(&self) -> Vec<EmailHistoryEntry> {
        if !self.history_path.exists() {
            return Vec::new();
        }
        match tokio::fs::read_to_string(&self.history_path).await {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(e) => {
                warn!("Failed to load email history: {}", e);
                Vec::new()
            }
        }
    }

    /// Append an entry and save to disk
    pub async fn record(&self, entry: EmailHistoryEntry) {
        let mut history = self.load().await;
        history.push(entry);
        // Keep only the last MAX_EMAIL_HISTORY entries
        if history.len() > MAX_EMAIL_HISTORY {
            history.drain(0..history.len() - MAX_EMAIL_HISTORY);
        }
        if let Ok(content) = serde_json::to_string_pretty(&history) {
            if let Some(parent) = self.history_path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            if let Err(e) = tokio::fs::write(&self.history_path, content).await {
                warn!("Failed to save email history: {}", e);
            }
        }
    }

    /// Get recent history (last N entries, newest first)
    pub async fn recent(&self, n: usize) -> Vec<EmailHistoryEntry> {
        let history = self.load().await;
        history.into_iter().rev().take(n).collect()
    }
}

/// Manages email sending
pub struct EmailManager {
    config: EmailConfig,
    version: String,
    history_manager: Option<EmailHistoryManager>,
}

impl EmailManager {
    /// Create a new email manager
    pub fn new(config: EmailConfig, version: impl Into<String>) -> Self {
        Self {
            config,
            version: version.into(),
            history_manager: None,
        }
    }

    /// Create a new email manager with history tracking
    pub fn with_history(mut self, lbhomedir: &Path) -> Self {
        self.history_manager = Some(EmailHistoryManager::new(lbhomedir));
        self
    }

    /// Check if email is enabled and configured
    pub fn is_configured(&self) -> bool {
        self.config.enabled
            && !self.config.smtp_host.is_empty()
            && !self.config.from_address.is_empty()
            && !self.config.notification_addresses.is_empty()
    }

    /// Send an email notification
    pub async fn send_notification(&self, email_type: &EmailType) -> Vec<SendResult> {
        if !self.is_configured() {
            warn!("Email not configured, skipping notification");
            return Vec::new();
        }

        let template = EmailTemplate::render(email_type, &self.version);
        let mut results = Vec::new();

        for recipient in &self.config.notification_addresses {
            let result = self
                .send_email(
                    recipient,
                    &template.subject,
                    &template.html_body,
                    &template.text_body,
                )
                .await;
            results.push(result);
        }

        // Record in history
        if let Some(ref hm) = self.history_manager {
            hm.record(EmailHistoryEntry {
                subject: template.subject.clone(),
                recipients: self.config.notification_addresses.clone(),
                results: results.clone(),
                sent_at: Utc::now(),
            })
            .await;
        }

        results
    }

    /// Send a test email to verify configuration
    pub async fn send_test(&self, recipient: &str) -> SendResult {
        let template = EmailTemplate::render(
            &EmailType::Custom {
                subject: "RustyLox Test Email".to_string(),
                body: "This is a test email from RustyLox. Your email configuration is working correctly.".to_string(),
            },
            &self.version,
        );

        let result = self
            .send_email(
                recipient,
                &template.subject,
                &template.html_body,
                &template.text_body,
            )
            .await;

        // Record in history
        if let Some(ref hm) = self.history_manager {
            hm.record(EmailHistoryEntry {
                subject: template.subject,
                recipients: vec![recipient.to_string()],
                results: vec![result.clone()],
                sent_at: Utc::now(),
            })
            .await;
        }

        result
    }

    /// Send a single email
    async fn send_email(
        &self,
        recipient: &str,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> SendResult {
        use lettre::{
            message::{header::ContentType, MultiPart, SinglePart},
            transport::smtp::authentication::Credentials,
            AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
        };

        debug!("Sending email to {}: {}", recipient, subject);

        let build_result =
            (|| -> std::result::Result<Message, Box<dyn std::error::Error + Send + Sync>> {
                let from =
                    format!("{} <{}>", self.config.from_name, self.config.from_address).parse()?;
                let to = recipient.parse()?;

                let message = Message::builder()
                    .from(from)
                    .to(to)
                    .subject(subject)
                    .multipart(
                        MultiPart::alternative()
                            .singlepart(
                                SinglePart::builder()
                                    .header(ContentType::TEXT_PLAIN)
                                    .body(text_body.to_string()),
                            )
                            .singlepart(
                                SinglePart::builder()
                                    .header(ContentType::TEXT_HTML)
                                    .body(html_body.to_string()),
                            ),
                    )?;

                Ok(message)
            })();

        let message = match build_result {
            Ok(m) => m,
            Err(e) => {
                let error_msg = format!("Failed to build email: {}", e);
                warn!("{}", error_msg);
                return SendResult {
                    success: false,
                    recipient: recipient.to_string(),
                    error: Some(error_msg),
                    sent_at: Utc::now(),
                };
            }
        };

        // Build SMTP transport
        let creds = Credentials::new(self.config.smtp_user.clone(), self.config.smtp_pass.clone());

        let send_result = if self.config.smtp_tls {
            let transport =
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.config.smtp_host)
                    .map_err(|e| format!("SMTP relay error: {}", e))
                    .map(|b| b.port(self.config.smtp_port).credentials(creds).build());

            match transport {
                Ok(t) => t.send(message).await.map_err(|e| e.to_string()),
                Err(e) => Err(e),
            }
        } else {
            let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&self.config.smtp_host)
                .map_err(|e| format!("SMTP relay error: {}", e))
                .map(|b| b.port(self.config.smtp_port).credentials(creds).build());

            match transport {
                Ok(t) => t.send(message).await.map_err(|e| e.to_string()),
                Err(e) => Err(e),
            }
        };

        match send_result {
            Ok(_) => {
                info!("Email sent successfully to {}", recipient);
                SendResult {
                    success: true,
                    recipient: recipient.to_string(),
                    error: None,
                    sent_at: Utc::now(),
                }
            }
            Err(e) => {
                warn!("Failed to send email to {}: {}", recipient, e);
                SendResult {
                    success: false,
                    recipient: recipient.to_string(),
                    error: Some(e),
                    sent_at: Utc::now(),
                }
            }
        }
    }
}
