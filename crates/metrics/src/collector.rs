//! System metrics collection using sysinfo

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use sysinfo::System;

/// Counters for application-level metrics
#[derive(Debug, Default)]
pub struct AppCounters {
    pub mqtt_messages_received: AtomicU64,
    pub mqtt_messages_sent: AtomicU64,
    pub http_requests_total: AtomicU64,
    pub http_requests_errors: AtomicU64,
}

impl AppCounters {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn inc_mqtt_received(&self) {
        self.mqtt_messages_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_mqtt_sent(&self) {
        self.mqtt_messages_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_http_requests(&self) {
        self.http_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_http_errors(&self) {
        self.http_requests_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> AppMetrics {
        AppMetrics {
            mqtt_messages_received: self.mqtt_messages_received.load(Ordering::Relaxed),
            mqtt_messages_sent: self.mqtt_messages_sent.load(Ordering::Relaxed),
            http_requests_total: self.http_requests_total.load(Ordering::Relaxed),
            http_requests_errors: self.http_requests_errors.load(Ordering::Relaxed),
        }
    }
}

/// Application-level metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMetrics {
    pub mqtt_messages_received: u64,
    pub mqtt_messages_sent: u64,
    pub http_requests_total: u64,
    pub http_requests_errors: u64,
}

/// System-level metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// CPU usage percentage (0-100)
    pub cpu_usage_percent: f32,
    /// Total physical memory in bytes
    pub memory_total_bytes: u64,
    /// Used memory in bytes
    pub memory_used_bytes: u64,
    /// Memory usage percentage (0-100)
    pub memory_usage_percent: f32,
    /// Total disk space in bytes
    pub disk_total_bytes: u64,
    /// Used disk space in bytes
    pub disk_used_bytes: u64,
    /// Disk usage percentage (0-100)
    pub disk_usage_percent: f32,
    /// System uptime in seconds
    pub uptime_seconds: u64,
    /// System hostname
    pub hostname: String,
    /// OS name
    pub os_name: String,
    /// OS version
    pub os_version: String,
    /// Kernel version
    pub kernel_version: String,
    /// Number of CPUs
    pub cpu_count: usize,
    /// Load average (1 min, 5 min, 15 min) - Linux/macOS only
    pub load_average: [f64; 3],
    /// Timestamp of measurement
    pub collected_at: DateTime<Utc>,
}

/// Collects system metrics
pub struct MetricsCollector {
    system: System,
    start_time: std::time::Instant,
    app_counters: Arc<AppCounters>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(app_counters: Arc<AppCounters>) -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self {
            system,
            start_time: std::time::Instant::now(),
            app_counters,
        }
    }

    /// Create with default (empty) counters
    pub fn with_default_counters() -> Self {
        Self::new(AppCounters::new())
    }

    /// Get application counters handle for incrementing
    pub fn counters(&self) -> Arc<AppCounters> {
        Arc::clone(&self.app_counters)
    }

    /// Collect current system metrics
    pub fn collect_system(&mut self) -> SystemMetrics {
        self.system.refresh_all();

        // CPU usage (average across all CPUs)
        let cpu_usage = if self.system.cpus().is_empty() {
            0.0
        } else {
            let total: f32 = self.system.cpus().iter().map(|c| c.cpu_usage()).sum();
            total / self.system.cpus().len() as f32
        };

        // Memory
        let memory_total = self.system.total_memory();
        let memory_used = self.system.used_memory();
        let memory_percent = if memory_total > 0 {
            (memory_used as f32 / memory_total as f32) * 100.0
        } else {
            0.0
        };

        // Disk - aggregate all disks
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let disk_total: u64 = disks.iter().map(|d| d.total_space()).sum();
        let disk_available: u64 = disks.iter().map(|d| d.available_space()).sum();
        let disk_used = disk_total.saturating_sub(disk_available);
        let disk_percent = if disk_total > 0 {
            (disk_used as f32 / disk_total as f32) * 100.0
        } else {
            0.0
        };

        // Load average
        let load = System::load_average();
        let load_average = [load.one, load.five, load.fifteen];

        SystemMetrics {
            cpu_usage_percent: cpu_usage,
            memory_total_bytes: memory_total,
            memory_used_bytes: memory_used,
            memory_usage_percent: memory_percent,
            disk_total_bytes: disk_total,
            disk_used_bytes: disk_used,
            disk_usage_percent: disk_percent,
            uptime_seconds: System::uptime(),
            hostname: System::host_name().unwrap_or_else(|| "unknown".to_string()),
            os_name: System::name().unwrap_or_else(|| "unknown".to_string()),
            os_version: System::os_version().unwrap_or_else(|| "unknown".to_string()),
            kernel_version: System::kernel_version().unwrap_or_else(|| "unknown".to_string()),
            cpu_count: self.system.cpus().len(),
            load_average,
            collected_at: Utc::now(),
        }
    }

    /// Get application metrics snapshot
    pub fn collect_app(&self) -> AppMetrics {
        self.app_counters.snapshot()
    }

    /// Get daemon uptime in seconds
    pub fn daemon_uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}
