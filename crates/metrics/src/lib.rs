//! Metrics and monitoring for RustyLox
//!
//! Provides:
//! - System metrics (CPU, memory, disk)
//! - Application metrics (MQTT messages, HTTP requests, plugins)
//! - Prometheus text format export
//! - Health check with component status

pub mod collector;
pub mod health;
pub mod prometheus;

pub use collector::{MetricsCollector, SystemMetrics};
pub use health::{ComponentStatus, HealthCheck, HealthStatus};
pub use prometheus::PrometheusExporter;
