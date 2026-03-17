//! Health check with component status

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Overall health status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Status of an individual component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
    pub checked_at: DateTime<Utc>,
}

impl ComponentStatus {
    pub fn ok(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Healthy,
            message: None,
            checked_at: Utc::now(),
        }
    }

    pub fn degraded(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Degraded,
            message: Some(message.into()),
            checked_at: Utc::now(),
        }
    }

    pub fn unhealthy(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Unhealthy,
            message: Some(message.into()),
            checked_at: Utc::now(),
        }
    }
}

/// Full health check report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Overall status (worst of all components)
    pub status: HealthStatus,
    /// Application version
    pub version: String,
    /// Daemon uptime in seconds
    pub uptime_seconds: u64,
    /// Uptime as human-readable string
    pub uptime_human: String,
    /// Component statuses
    pub components: Vec<ComponentStatus>,
    /// Quick metrics summary
    pub metrics: HealthMetrics,
    /// Timestamp
    pub checked_at: DateTime<Utc>,
}

/// Quick metrics for health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub cpu_usage_percent: f32,
    pub memory_usage_percent: f32,
    pub disk_usage_percent: f32,
    pub mqtt_connected: bool,
    pub active_plugins: usize,
    pub miniserver_count: usize,
}

impl HealthCheck {
    /// Build a health check from components
    pub fn build(
        version: String,
        uptime_seconds: u64,
        components: Vec<ComponentStatus>,
        metrics: HealthMetrics,
    ) -> Self {
        let status = overall_status(&components);
        Self {
            status,
            version,
            uptime_seconds,
            uptime_human: format_uptime(uptime_seconds),
            components,
            metrics,
            checked_at: Utc::now(),
        }
    }
}

/// Calculate overall health from components
fn overall_status(components: &[ComponentStatus]) -> HealthStatus {
    if components
        .iter()
        .any(|c| c.status == HealthStatus::Unhealthy)
    {
        HealthStatus::Unhealthy
    } else if components
        .iter()
        .any(|c| c.status == HealthStatus::Degraded)
    {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    }
}

/// Format uptime as human-readable string
pub fn format_uptime(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else if seconds < 86400 {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        format!("{}h {}m", hours, minutes)
    } else {
        let days = seconds / 86400;
        let hours = (seconds % 86400) / 3600;
        format!("{}d {}h", days, hours)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_uptime() {
        assert_eq!(format_uptime(30), "30s");
        assert_eq!(format_uptime(90), "1m 30s");
        assert_eq!(format_uptime(3661), "1h 1m");
        assert_eq!(format_uptime(90000), "1d 1h");
    }

    #[test]
    fn test_overall_status() {
        let healthy = vec![ComponentStatus::ok("a"), ComponentStatus::ok("b")];
        assert_eq!(overall_status(&healthy), HealthStatus::Healthy);

        let degraded = vec![
            ComponentStatus::ok("a"),
            ComponentStatus::degraded("b", "warn"),
        ];
        assert_eq!(overall_status(&degraded), HealthStatus::Degraded);

        let unhealthy = vec![
            ComponentStatus::ok("a"),
            ComponentStatus::unhealthy("b", "error"),
        ];
        assert_eq!(overall_status(&unhealthy), HealthStatus::Unhealthy);
    }
}
