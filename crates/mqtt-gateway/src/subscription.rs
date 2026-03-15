//! MQTT subscription management

use loxberry_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;
use tokio::fs;
use tracing::{debug, info};

/// MQTT subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    /// Topic filter (supports wildcards + and #)
    pub topic: String,

    /// Human-readable name
    pub name: Option<String>,

    /// Whether subscription is enabled
    pub enabled: bool,

    /// Plugin that owns this subscription
    pub plugin: Option<String>,
}

/// Subscription manager
pub struct SubscriptionManager {
    config_dir: PathBuf,
    subscriptions: RwLock<Vec<Subscription>>,
}

impl SubscriptionManager {
    /// Create a new subscription manager
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            config_dir,
            subscriptions: RwLock::new(Vec::new()),
        }
    }

    /// Load subscriptions from disk
    pub async fn load(&self) -> Result<()> {
        let subscriptions_file = self.config_dir.join("mqtt_subscriptions.cfg");

        if !subscriptions_file.exists() {
            info!("No subscriptions file found, using empty list");
            return Ok(());
        }

        let content = fs::read_to_string(&subscriptions_file)
            .await
            .map_err(|e| Error::gateway(format!("Failed to read subscriptions: {}", e)))?;

        let subscriptions = self.parse_subscriptions_ini(&content)?;

        let mut subs = self.subscriptions.write().unwrap();
        *subs = subscriptions;

        info!(
            "Loaded {} subscriptions from {}",
            subs.len(),
            subscriptions_file.display()
        );

        Ok(())
    }

    /// Parse subscriptions from INI format
    fn parse_subscriptions_ini(&self, content: &str) -> Result<Vec<Subscription>> {
        let mut subscriptions = Vec::new();
        let mut current_section: Option<String> = None;
        let mut current_topic: Option<String> = None;
        let mut current_enabled = true;
        let mut current_name: Option<String> = None;
        let mut current_plugin: Option<String> = None;

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            // Section header
            if line.starts_with('[') && line.ends_with(']') {
                // Save previous subscription if exists
                if let Some(topic) = current_topic.take() {
                    subscriptions.push(Subscription {
                        topic,
                        name: current_name.take(),
                        enabled: current_enabled,
                        plugin: current_plugin.take(),
                    });
                }

                current_section = Some(line[1..line.len() - 1].to_string());
                current_enabled = true;
                continue;
            }

            // Key=value
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_uppercase();
                let value = value.trim();

                match key.as_str() {
                    "TOPIC" => current_topic = Some(value.to_string()),
                    "NAME" => current_name = Some(value.to_string()),
                    "ENABLED" => current_enabled = value == "1" || value.to_lowercase() == "true",
                    "PLUGIN" => current_plugin = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        // Save last subscription
        if let Some(topic) = current_topic {
            subscriptions.push(Subscription {
                topic,
                name: current_name,
                enabled: current_enabled,
                plugin: current_plugin,
            });
        }

        Ok(subscriptions)
    }

    /// Get all enabled subscriptions
    pub fn get_all(&self) -> Vec<Subscription> {
        let subs = self.subscriptions.read().unwrap();
        subs.iter().filter(|s| s.enabled).cloned().collect()
    }

    /// Count total subscriptions
    pub fn count(&self) -> usize {
        self.subscriptions.read().unwrap().len()
    }

    /// Add a subscription
    pub fn add(&self, subscription: Subscription) {
        let mut subs = self.subscriptions.write().unwrap();
        subs.push(subscription);
    }

    /// Remove subscriptions by topic
    pub fn remove(&self, topic: &str) {
        let mut subs = self.subscriptions.write().unwrap();
        subs.retain(|s| s.topic != topic);
    }

    /// Check if a topic matches any subscription
    pub fn matches(&self, topic: &str) -> bool {
        let subs = self.subscriptions.read().unwrap();
        subs.iter()
            .filter(|s| s.enabled)
            .any(|s| topic_matches(&s.topic, topic))
    }
}

/// Check if a topic matches a subscription filter (with wildcards)
fn topic_matches(filter: &str, topic: &str) -> bool {
    // Exact match
    if filter == topic {
        return true;
    }

    let filter_parts: Vec<&str> = filter.split('/').collect();
    let topic_parts: Vec<&str> = topic.split('/').collect();

    // # wildcard matches everything from this level down
    if filter.ends_with("/#") {
        let prefix = &filter[..filter.len() - 2];
        return topic.starts_with(prefix);
    }

    // + wildcard matches single level
    if filter_parts.len() != topic_parts.len() {
        return false;
    }

    for (filter_part, topic_part) in filter_parts.iter().zip(topic_parts.iter()) {
        if *filter_part == "+" {
            continue; // Matches any single level
        }
        if filter_part != topic_part {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_matching() {
        assert!(topic_matches("home/sensor", "home/sensor"));
        assert!(topic_matches(
            "home/+/temperature",
            "home/bedroom/temperature"
        ));
        assert!(topic_matches("home/#", "home/bedroom/temperature"));
        assert!(topic_matches("home/#", "home/kitchen/humidity/value"));

        assert!(!topic_matches("home/sensor", "home/other"));
        assert!(!topic_matches("home/+/temp", "home/bedroom/humidity"));
        assert!(!topic_matches("office/#", "home/sensor"));
    }

    #[test]
    fn test_parse_subscriptions() {
        let ini = r#"
[Subscription1]
TOPIC=home/sensor/+/temperature
NAME=All Temperature Sensors
ENABLED=1
PLUGIN=weatherplugin

[Subscription2]
TOPIC=home/lights/#
ENABLED=0
"#;

        let manager = SubscriptionManager::new(PathBuf::from("/tmp"));
        let subs = manager.parse_subscriptions_ini(ini).unwrap();

        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0].topic, "home/sensor/+/temperature");
        assert!(subs[0].enabled);
        assert_eq!(subs[0].plugin, Some("weatherplugin".to_string()));

        assert_eq!(subs[1].topic, "home/lights/#");
        assert!(!subs[1].enabled);
    }
}
