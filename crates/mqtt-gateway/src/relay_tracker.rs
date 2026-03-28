//! Relay state tracker for MQTT Gateway "Incoming Overview"
//!
//! Tracks per-topic relay state so the monitor UI can display what was sent
//! to the Miniserver, HTTP response codes, filter status, and timestamps —
//! matching the original LoxBerry "Incoming Overview" page.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Per-topic relay state for HTTP virtual inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRelayEntry {
    /// Virtual input name (topic with slashes replaced by underscores)
    pub virtual_input: String,
    /// Original MQTT topic
    pub original_topic: String,
    /// Last value sent
    pub last_value: String,
    /// Unix timestamp of last arrival
    pub timestamp: u64,
    /// Per-Miniserver relay results
    pub relay_results: Vec<MsRelayResult>,
    /// If filtered by regex, which filter line matched (0 = not filtered)
    pub regex_filter_line: Option<u32>,
    /// Whether "do not forward" is set for this topic
    pub do_not_forward: bool,
}

/// Per-Miniserver HTTP relay result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsRelayResult {
    /// Miniserver ID
    pub ms_id: String,
    /// HTTP status code (200, 404, 500, 0=not sent yet)
    pub code: u16,
    /// Unix timestamp of last send
    pub last_sent: u64,
}

/// Per-topic relay state for UDP transmissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpRelayEntry {
    /// Topic name (used as key in UDP message)
    pub topic: String,
    /// Original MQTT topic
    pub original_topic: String,
    /// Last value
    pub last_value: String,
    /// Unix timestamp of last arrival
    pub timestamp: u64,
    /// If filtered by regex, which filter line matched
    pub regex_filter_line: Option<u32>,
    /// Whether "do not forward" is set for this topic
    pub do_not_forward: bool,
}

/// Per-topic settings (persisted to gateway config)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopicSettings {
    /// Disable caching (always send, even if value unchanged)
    #[serde(default)]
    pub disable_cache: bool,
    /// Reset value to 0 after sending to Miniserver
    #[serde(default)]
    pub reset_after_send: bool,
    /// Do not forward this topic to Miniserver
    #[serde(default)]
    pub do_not_forward: bool,
}

/// Full response for the "Incoming Overview" / relayed topics endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayedTopicsResponse {
    pub http: Vec<HttpRelayEntry>,
    pub udp: Vec<UdpRelayEntry>,
    pub http_count: u64,
    pub udp_count: u64,
    pub topic_settings: std::collections::HashMap<String, TopicSettings>,
}

/// Tracks relay state for all topics passing through the gateway
pub struct RelayTracker {
    /// HTTP relay entries keyed by virtual input name
    http_entries: DashMap<String, HttpRelayEntry>,
    /// UDP relay entries keyed by topic
    udp_entries: DashMap<String, UdpRelayEntry>,
    /// Per-topic settings keyed by normalized topic (underscores)
    topic_settings: DashMap<String, TopicSettings>,
    /// HTTP relayed count
    http_relayed_count: AtomicU64,
    /// UDP relayed count
    udp_relayed_count: AtomicU64,
}

impl RelayTracker {
    /// Create a new relay tracker
    pub fn new() -> Self {
        Self {
            http_entries: DashMap::new(),
            udp_entries: DashMap::new(),
            topic_settings: DashMap::new(),
            http_relayed_count: AtomicU64::new(0),
            udp_relayed_count: AtomicU64::new(0),
        }
    }

    /// Record an HTTP relay event (topic received from MQTT, sent to Miniserver via HTTP)
    pub fn record_http_relay(&self, topic: &str, value: &str, ms_id: &str, http_code: u16) {
        let virtual_input = topic.replace('/', "_");
        let now = now_unix();

        self.http_entries
            .entry(virtual_input.clone())
            .and_modify(|entry| {
                entry.last_value = value.to_string();
                entry.timestamp = now;
                // Update or add MS relay result
                if let Some(result) = entry.relay_results.iter_mut().find(|r| r.ms_id == ms_id) {
                    result.code = http_code;
                    result.last_sent = now;
                } else {
                    entry.relay_results.push(MsRelayResult {
                        ms_id: ms_id.to_string(),
                        code: http_code,
                        last_sent: now,
                    });
                }
            })
            .or_insert_with(|| {
                self.http_relayed_count.fetch_add(1, Ordering::Relaxed);
                HttpRelayEntry {
                    virtual_input: virtual_input.clone(),
                    original_topic: topic.to_string(),
                    last_value: value.to_string(),
                    timestamp: now,
                    relay_results: vec![MsRelayResult {
                        ms_id: ms_id.to_string(),
                        code: http_code,
                        last_sent: now,
                    }],
                    regex_filter_line: None,
                    do_not_forward: self
                        .topic_settings
                        .get(&virtual_input)
                        .map(|s| s.do_not_forward)
                        .unwrap_or(false),
                }
            });
    }

    /// Record an HTTP topic that was received but not yet sent (cached)
    pub fn record_http_cached(&self, topic: &str, value: &str) {
        let virtual_input = topic.replace('/', "_");
        let now = now_unix();

        self.http_entries
            .entry(virtual_input.clone())
            .and_modify(|entry| {
                entry.last_value = value.to_string();
                entry.timestamp = now;
            })
            .or_insert_with(|| {
                self.http_relayed_count.fetch_add(1, Ordering::Relaxed);
                HttpRelayEntry {
                    virtual_input: virtual_input.clone(),
                    original_topic: topic.to_string(),
                    last_value: value.to_string(),
                    timestamp: now,
                    relay_results: vec![],
                    regex_filter_line: None,
                    do_not_forward: self
                        .topic_settings
                        .get(&virtual_input)
                        .map(|s| s.do_not_forward)
                        .unwrap_or(false),
                }
            });
    }

    /// Record a topic that was filtered by regex
    pub fn record_filtered(&self, topic: &str, value: &str, filter_line: u32) {
        let virtual_input = topic.replace('/', "_");
        let now = now_unix();

        self.http_entries
            .entry(virtual_input.clone())
            .and_modify(|entry| {
                entry.last_value = value.to_string();
                entry.timestamp = now;
                entry.regex_filter_line = Some(filter_line);
            })
            .or_insert_with(|| {
                self.http_relayed_count.fetch_add(1, Ordering::Relaxed);
                HttpRelayEntry {
                    virtual_input: virtual_input.clone(),
                    original_topic: topic.to_string(),
                    last_value: value.to_string(),
                    timestamp: now,
                    relay_results: vec![],
                    regex_filter_line: Some(filter_line),
                    do_not_forward: false,
                }
            });
    }

    /// Record a UDP relay event
    pub fn record_udp_relay(&self, topic: &str, original_topic: &str, value: &str) {
        let now = now_unix();

        self.udp_entries
            .entry(topic.to_string())
            .and_modify(|entry| {
                entry.last_value = value.to_string();
                entry.timestamp = now;
            })
            .or_insert_with(|| {
                self.udp_relayed_count.fetch_add(1, Ordering::Relaxed);
                UdpRelayEntry {
                    topic: topic.to_string(),
                    original_topic: original_topic.to_string(),
                    last_value: value.to_string(),
                    timestamp: now,
                    regex_filter_line: None,
                    do_not_forward: self
                        .topic_settings
                        .get(&topic.replace('/', "_"))
                        .map(|s| s.do_not_forward)
                        .unwrap_or(false),
                }
            });
    }

    /// Get all relayed topics for the "Incoming Overview"
    pub fn get_relayed_topics(&self) -> RelayedTopicsResponse {
        let http: Vec<HttpRelayEntry> = self
            .http_entries
            .iter()
            .map(|entry| {
                let mut e = entry.value().clone();
                // Reflect current do_not_forward setting
                let vi = &e.virtual_input;
                e.do_not_forward = self
                    .topic_settings
                    .get(vi)
                    .map(|s| s.do_not_forward)
                    .unwrap_or(false);
                e
            })
            .collect();

        let udp: Vec<UdpRelayEntry> = self
            .udp_entries
            .iter()
            .map(|entry| {
                let mut e = entry.value().clone();
                let key = e.topic.replace('/', "_");
                e.do_not_forward = self
                    .topic_settings
                    .get(&key)
                    .map(|s| s.do_not_forward)
                    .unwrap_or(false);
                e
            })
            .collect();

        let topic_settings: std::collections::HashMap<String, TopicSettings> = self
            .topic_settings
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        RelayedTopicsResponse {
            http_count: http.len() as u64,
            udp_count: udp.len() as u64,
            http,
            udp,
            topic_settings,
        }
    }

    /// Check if a topic should not be forwarded
    pub fn is_do_not_forward(&self, topic: &str) -> bool {
        let normalized = topic.replace('/', "_");
        self.topic_settings
            .get(&normalized)
            .map(|s| s.do_not_forward)
            .unwrap_or(false)
    }

    /// Check if caching is disabled for a topic
    pub fn is_cache_disabled(&self, topic: &str) -> bool {
        let normalized = topic.replace('/', "_");
        self.topic_settings
            .get(&normalized)
            .map(|s| s.disable_cache)
            .unwrap_or(false)
    }

    /// Check if reset-after-send is enabled for a topic
    pub fn is_reset_after_send(&self, topic: &str) -> bool {
        let normalized = topic.replace('/', "_");
        self.topic_settings
            .get(&normalized)
            .map(|s| s.reset_after_send)
            .unwrap_or(false)
    }

    /// Update a topic setting
    pub fn update_topic_setting(&self, normalized_topic: &str, setting: &str, enabled: bool) {
        self.topic_settings
            .entry(normalized_topic.to_string())
            .and_modify(|s| match setting {
                "disable_cache" => s.disable_cache = enabled,
                "reset_after_send" => s.reset_after_send = enabled,
                "do_not_forward" => s.do_not_forward = enabled,
                _ => {}
            })
            .or_insert_with(|| {
                let mut s = TopicSettings::default();
                match setting {
                    "disable_cache" => s.disable_cache = enabled,
                    "reset_after_send" => s.reset_after_send = enabled,
                    "do_not_forward" => s.do_not_forward = enabled,
                    _ => {}
                }
                s
            });
    }

    /// Delete a topic from the tracker cache
    pub fn delete_topic(&self, topic: &str) {
        let normalized = topic.replace('/', "_");
        self.http_entries.remove(&normalized);
        self.udp_entries.remove(topic);
    }

    /// Clear all tracked entries
    pub fn clear(&self) {
        self.http_entries.clear();
        self.udp_entries.clear();
        self.http_relayed_count.store(0, Ordering::Relaxed);
        self.udp_relayed_count.store(0, Ordering::Relaxed);
    }
}

impl Default for RelayTracker {
    fn default() -> Self {
        Self::new()
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
