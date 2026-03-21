//! Email templates for system notifications

use chrono::{DateTime, Utc};

/// Type of notification email
#[derive(Debug, Clone)]
pub enum EmailType {
    Alert {
        severity: String,
        title: String,
        message: String,
    },
    PluginInstalled {
        name: String,
        version: String,
    },
    PluginUninstalled {
        name: String,
    },
    BackupCompleted {
        filename: String,
        size_bytes: u64,
        success: bool,
    },
    MiniserverDisconnected {
        miniserver_name: String,
    },
    MiniserverReconnected {
        miniserver_name: String,
    },
    Custom {
        subject: String,
        body: String,
    },
}

/// Rendered email with subject and HTML body
pub struct EmailTemplate {
    pub subject: String,
    pub html_body: String,
    pub text_body: String,
}

impl EmailTemplate {
    /// Render an email template for the given event type
    pub fn render(email_type: &EmailType, version: &str) -> Self {
        let (subject, title, message, severity_class) = match email_type {
            EmailType::Alert {
                severity,
                title,
                message,
            } => {
                let class = match severity.as_str() {
                    "critical" => "color: #dc3545",
                    "warning" => "color: #ffc107",
                    _ => "color: #17a2b8",
                };
                (
                    format!("[RustyLox] Alert: {}", title),
                    title.clone(),
                    message.clone(),
                    class.to_string(),
                )
            }
            EmailType::PluginInstalled { name, version: ver } => (
                format!("[RustyLox] Plugin installed: {}", name),
                format!("Plugin Installed: {}", name),
                format!(
                    "Plugin '{}' version {} was successfully installed.",
                    name, ver
                ),
                "color: #28a745".to_string(),
            ),
            EmailType::PluginUninstalled { name } => (
                format!("[RustyLox] Plugin uninstalled: {}", name),
                format!("Plugin Uninstalled: {}", name),
                format!("Plugin '{}' was uninstalled.", name),
                "color: #6c757d".to_string(),
            ),
            EmailType::BackupCompleted {
                filename,
                size_bytes,
                success,
            } => {
                let status = if *success { "completed" } else { "failed" };
                let size_mb = *size_bytes as f64 / 1_048_576.0;
                (
                    format!("[RustyLox] Backup {}", status),
                    format!("Backup {}", status),
                    format!("Backup '{}' {} ({:.1} MB).", filename, status, size_mb),
                    if *success {
                        "color: #28a745".to_string()
                    } else {
                        "color: #dc3545".to_string()
                    },
                )
            }
            EmailType::MiniserverDisconnected { miniserver_name } => (
                format!("[RustyLox] Miniserver disconnected: {}", miniserver_name),
                "Miniserver Disconnected".to_string(),
                format!("Miniserver '{}' is no longer reachable.", miniserver_name),
                "color: #dc3545".to_string(),
            ),
            EmailType::MiniserverReconnected { miniserver_name } => (
                format!("[RustyLox] Miniserver reconnected: {}", miniserver_name),
                "Miniserver Reconnected".to_string(),
                format!("Miniserver '{}' is back online.", miniserver_name),
                "color: #28a745".to_string(),
            ),
            EmailType::Custom { subject, body } => {
                return Self {
                    subject: subject.clone(),
                    html_body: render_html_email(subject, body, "", version),
                    text_body: format!("{}\n\n{}", subject, body),
                };
            }
        };

        let html_body = render_html_email(&title, &message, &severity_class, version);
        let text_body = format!("{}\n\n{}\n\nSent by RustyLox v{}", title, message, version);

        Self {
            subject,
            html_body,
            text_body,
        }
    }
}

/// Render the standard HTML email layout
fn render_html_email(title: &str, message: &str, title_style: &str, version: &str) -> String {
    let now: DateTime<Utc> = Utc::now();
    let timestamp = now.format("%Y-%m-%d %H:%M:%S UTC").to_string();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
</head>
<body style="margin: 0; padding: 0; font-family: Arial, sans-serif; background-color: #f5f5f5;">
    <table width="100%" cellpadding="0" cellspacing="0" style="background-color: #f5f5f5; padding: 20px 0;">
        <tr>
            <td align="center">
                <table width="600" cellpadding="0" cellspacing="0" style="background-color: #ffffff; border-radius: 8px; overflow: hidden; box-shadow: 0 2px 8px rgba(0,0,0,0.1);">
                    <!-- Header -->
                    <tr>
                        <td style="background-color: #ff6b35; padding: 24px 32px;">
                            <h1 style="margin: 0; color: white; font-size: 22px;">RustyLox Notification</h1>
                        </td>
                    </tr>
                    <!-- Content -->
                    <tr>
                        <td style="padding: 32px;">
                            <h2 style="margin: 0 0 16px 0; font-size: 18px; {title_style}">{title}</h2>
                            <p style="margin: 0 0 24px 0; color: #333; line-height: 1.6; font-size: 15px;">{message}</p>
                            <p style="margin: 0; color: #666; font-size: 13px;">
                                <strong>Time:</strong> {timestamp}
                            </p>
                        </td>
                    </tr>
                    <!-- Footer -->
                    <tr>
                        <td style="background-color: #f8f9fa; padding: 16px 32px; border-top: 1px solid #e9ecef;">
                            <p style="margin: 0; color: #6c757d; font-size: 12px; text-align: center;">
                                RustyLox v{version} &mdash; Smart Home Platform
                            </p>
                        </td>
                    </tr>
                </table>
            </td>
        </tr>
    </table>
</body>
</html>"#,
        title = title,
        message = message,
        title_style = title_style,
        timestamp = timestamp,
        version = version
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_alert_email() {
        let email_type = EmailType::Alert {
            severity: "warning".to_string(),
            title: "High CPU Usage".to_string(),
            message: "CPU usage is above 90%".to_string(),
        };
        let template = EmailTemplate::render(&email_type, "1.0.0");
        assert!(template.subject.contains("High CPU Usage"));
        assert!(template.html_body.contains("High CPU Usage"));
        assert!(template.html_body.contains("CPU usage is above 90%"));
    }

    #[test]
    fn test_render_plugin_installed_email() {
        let email_type = EmailType::PluginInstalled {
            name: "TestPlugin".to_string(),
            version: "1.2.3".to_string(),
        };
        let template = EmailTemplate::render(&email_type, "1.0.0");
        assert!(template.subject.contains("TestPlugin"));
        assert!(template.html_body.contains("1.2.3"));
    }
}
