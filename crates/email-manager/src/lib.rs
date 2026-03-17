//! Email Manager - SMTP-based email notifications for RustyLox
//!
//! Provides:
//! - SMTP client configuration (TLS, auth)
//! - HTML email templates for system events
//! - Async email sending via lettre

pub mod config;
pub mod smtp;
pub mod templates;

pub use config::{EmailConfig, EmailConfigManager};
pub use smtp::{EmailManager, SendResult};
pub use templates::{EmailTemplate, EmailType};
