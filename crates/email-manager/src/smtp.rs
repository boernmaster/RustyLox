//! SMTP email client using lettre

use crate::config::EmailConfig;
use crate::templates::{EmailTemplate, EmailType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

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

/// Manages email sending
pub struct EmailManager {
    config: EmailConfig,
    version: String,
}

impl EmailManager {
    /// Create a new email manager
    pub fn new(config: EmailConfig, version: impl Into<String>) -> Self {
        Self {
            config,
            version: version.into(),
        }
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
            let result = self.send_email(recipient, &template.subject, &template.html_body, &template.text_body).await;
            results.push(result);
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

        self.send_email(recipient, &template.subject, &template.html_body, &template.text_body).await
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

        let build_result = (|| -> std::result::Result<Message, Box<dyn std::error::Error + Send + Sync>> {
            let from = format!("{} <{}>", self.config.from_name, self.config.from_address)
                .parse()?;
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
        let creds = Credentials::new(
            self.config.smtp_user.clone(),
            self.config.smtp_pass.clone(),
        );

        let send_result = if self.config.smtp_tls {
            let transport =
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.config.smtp_host)
                    .map_err(|e| format!("SMTP relay error: {}", e))
                    .map(|b| {
                        b.port(self.config.smtp_port)
                            .credentials(creds)
                            .build()
                    });

            match transport {
                Ok(t) => t.send(message).await.map_err(|e| e.to_string()),
                Err(e) => Err(e),
            }
        } else {
            let transport =
                AsyncSmtpTransport::<Tokio1Executor>::relay(&self.config.smtp_host)
                    .map_err(|e| format!("SMTP relay error: {}", e))
                    .map(|b| {
                        b.port(self.config.smtp_port)
                            .credentials(creds)
                            .build()
                    });

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
