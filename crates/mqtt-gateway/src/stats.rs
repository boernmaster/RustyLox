//! MQTT Gateway Statistics Tracking

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

/// Statistics for MQTT Gateway operations
#[derive(Debug)]
pub struct MqttGatewayStats {
    /// Total messages received from MQTT broker
    pub messages_received: AtomicU64,
    /// Messages successfully relayed to Miniserver
    pub messages_relayed: AtomicU64,
    /// Messages filtered (not relayed due to filters)
    pub messages_filtered: AtomicU64,
    /// Parameters accepted by Miniserver
    pub miniserver_accepted: AtomicU64,
    /// Parameters rejected by Miniserver
    pub miniserver_rejected: AtomicU64,
    /// Rejected parameter names and their counts
    pub rejected_params: DashMap<String, RejectedParam>,
    /// Timestamp when stats started
    pub started_at: SystemTime,
}

/// Information about a rejected parameter
#[derive(Debug, Clone)]
pub struct RejectedParam {
    /// Number of times this parameter was rejected
    pub count: u64,
    /// Last time this parameter was rejected
    pub last_seen: SystemTime,
    /// Last value attempted
    pub last_value: String,
}

impl MqttGatewayStats {
    /// Create new statistics tracker
    pub fn new() -> Self {
        Self {
            messages_received: AtomicU64::new(0),
            messages_relayed: AtomicU64::new(0),
            messages_filtered: AtomicU64::new(0),
            miniserver_accepted: AtomicU64::new(0),
            miniserver_rejected: AtomicU64::new(0),
            rejected_params: DashMap::new(),
            started_at: SystemTime::now(),
        }
    }

    /// Increment messages received counter
    pub fn inc_received(&self) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment messages relayed counter
    pub fn inc_relayed(&self) {
        self.messages_relayed.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment messages filtered counter
    pub fn inc_filtered(&self) {
        self.messages_filtered.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a parameter accepted by Miniserver
    pub fn record_accepted(&self) {
        self.miniserver_accepted.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a parameter rejected by Miniserver
    pub fn record_rejected(&self, param_name: String, value: String) {
        self.miniserver_rejected.fetch_add(1, Ordering::Relaxed);

        self.rejected_params
            .entry(param_name)
            .and_modify(|p| {
                p.count += 1;
                p.last_seen = SystemTime::now();
                p.last_value = value.clone();
            })
            .or_insert(RejectedParam {
                count: 1,
                last_seen: SystemTime::now(),
                last_value: value,
            });
    }

    /// Get current statistics snapshot
    pub fn snapshot(&self) -> StatsSnapshot {
        let uptime = self.started_at.elapsed().unwrap_or_default();

        StatsSnapshot {
            messages_received: self.messages_received.load(Ordering::Relaxed),
            messages_relayed: self.messages_relayed.load(Ordering::Relaxed),
            messages_filtered: self.messages_filtered.load(Ordering::Relaxed),
            miniserver_accepted: self.miniserver_accepted.load(Ordering::Relaxed),
            miniserver_rejected: self.miniserver_rejected.load(Ordering::Relaxed),
            uptime_seconds: uptime.as_secs(),
        }
    }

    /// Get top N rejected parameters
    pub fn top_rejected(&self, n: usize) -> Vec<(String, RejectedParam)> {
        let mut params: Vec<(String, RejectedParam)> = self
            .rejected_params
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        params.sort_by_key(|b| std::cmp::Reverse(b.1.count));
        params.truncate(n);
        params
    }

    /// Reset all statistics
    pub fn reset(&self) {
        self.messages_received.store(0, Ordering::Relaxed);
        self.messages_relayed.store(0, Ordering::Relaxed);
        self.messages_filtered.store(0, Ordering::Relaxed);
        self.miniserver_accepted.store(0, Ordering::Relaxed);
        self.miniserver_rejected.store(0, Ordering::Relaxed);
        self.rejected_params.clear();
    }
}

impl Default for MqttGatewayStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics snapshot (for serialization)
#[derive(Debug, Clone, serde::Serialize)]
pub struct StatsSnapshot {
    pub messages_received: u64,
    pub messages_relayed: u64,
    pub messages_filtered: u64,
    pub miniserver_accepted: u64,
    pub miniserver_rejected: u64,
    pub uptime_seconds: u64,
}

impl StatsSnapshot {
    /// Calculate success rate percentage
    pub fn success_rate(&self) -> f64 {
        let total = self.miniserver_accepted + self.miniserver_rejected;
        if total == 0 {
            return 100.0;
        }
        (self.miniserver_accepted as f64 / total as f64) * 100.0
    }

    /// Calculate messages per second
    pub fn messages_per_second(&self) -> f64 {
        if self.uptime_seconds == 0 {
            return 0.0;
        }
        self.messages_received as f64 / self.uptime_seconds as f64
    }
}
