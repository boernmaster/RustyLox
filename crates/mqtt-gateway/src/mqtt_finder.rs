//! MQTT Finder — stores all MQTT messages seen on the broker
//!
//! Unlike the RelayTracker (which only tracks messages relayed to the Miniserver),
//! the Finder records *every* MQTT message with its last payload and timestamp,
//! matching the original LoxBerry "MQTT Finder" feature.
//! Entries older than 7 days are automatically cleaned up.

use dashmap::DashMap;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Retention period: 7 days in seconds
const RETENTION_SECS: u64 = 7 * 24 * 3600;

/// A single topic entry in the finder
#[derive(Debug, Clone, Serialize)]
pub struct FinderEntry {
    /// MQTT topic
    pub topic: String,
    /// Last payload (as string)
    pub payload: String,
    /// Unix timestamp (seconds) of last message
    pub timestamp: f64,
}

/// Stores all MQTT topics seen on the broker
pub struct MqttFinder {
    /// topic → (payload, timestamp)
    entries: DashMap<String, (String, f64)>,
    /// Total message count since start
    total_messages: AtomicU64,
}

impl MqttFinder {
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
            total_messages: AtomicU64::new(0),
        }
    }

    /// Record an incoming MQTT message
    pub fn record(&self, topic: &str, payload: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        self.entries
            .entry(topic.to_string())
            .and_modify(|(p, t)| {
                *p = payload.to_string();
                *t = now;
            })
            .or_insert_with(|| (payload.to_string(), now));

        self.total_messages.fetch_add(1, Ordering::Relaxed);
    }

    /// Get all entries sorted by topic
    pub fn get_all(&self) -> Vec<FinderEntry> {
        let mut entries: Vec<FinderEntry> = self
            .entries
            .iter()
            .map(|e| FinderEntry {
                topic: e.key().clone(),
                payload: e.value().0.clone(),
                timestamp: e.value().1,
            })
            .collect();

        entries.sort_by(|a, b| a.topic.cmp(&b.topic));
        entries
    }

    /// Get total number of unique topics
    pub fn topic_count(&self) -> usize {
        self.entries.len()
    }

    /// Get total messages received since start
    pub fn total_messages(&self) -> u64 {
        self.total_messages.load(Ordering::Relaxed)
    }

    /// Remove entries older than 7 days
    pub fn cleanup(&self) -> usize {
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
            - RETENTION_SECS as f64;

        let before = self.entries.len();
        self.entries.retain(|_, (_, ts)| *ts > cutoff);
        before - self.entries.len()
    }
}

impl Default for MqttFinder {
    fn default() -> Self {
        Self::new()
    }
}
