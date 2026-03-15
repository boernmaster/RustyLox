//! Error types for LoxBerry

use std::fmt;

/// Result type alias for LoxBerry operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for LoxBerry
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Miniserver error
    #[error("Miniserver error: {0}")]
    Miniserver(String),

    /// Plugin error
    #[error("Plugin error: {0}")]
    Plugin(String),

    /// MQTT error
    #[error("MQTT error: {0}")]
    Mqtt(String),

    /// MQTT Gateway error
    #[error("Gateway error: {0}")]
    Gateway(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Error::Config(msg.into())
    }

    /// Create a network error
    pub fn network(msg: impl Into<String>) -> Self {
        Error::Network(msg.into())
    }

    /// Create a Miniserver error
    pub fn miniserver(msg: impl Into<String>) -> Self {
        Error::Miniserver(msg.into())
    }

    /// Create a plugin error
    pub fn plugin(msg: impl Into<String>) -> Self {
        Error::Plugin(msg.into())
    }

    /// Create an MQTT error
    pub fn mqtt(msg: impl Into<String>) -> Self {
        Error::Mqtt(msg.into())
    }

    /// Create a gateway error
    pub fn gateway(msg: impl Into<String>) -> Self {
        Error::Gateway(msg.into())
    }

    /// Create a generic error
    pub fn other(msg: impl Into<String>) -> Self {
        Error::Other(msg.into())
    }
}
