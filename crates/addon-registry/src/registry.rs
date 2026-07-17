//! In-memory registry of self-registered addon instances.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::model::{AddonInstance, AddonInstanceView};

/// An instance is considered offline after this many missed heartbeats
/// (heartbeat interval is 60s, so 3 missed = ~180s of silence).
const OFFLINE_AFTER_MISSED_HEARTBEATS: i64 = 3;
const HEARTBEAT_INTERVAL_SECONDS: i64 = 60;

pub struct Registry {
    instances: Arc<Mutex<HashMap<String, AddonInstance>>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register (or re-register) an instance. Last-write-wins by name -
    /// handles container restarts / IP changes on the LAN.
    pub async fn register(&self, instance: AddonInstance) {
        let mut guard = self.instances.lock().await;
        guard.insert(instance.name.clone(), instance);
    }

    pub async fn list(&self, now: DateTime<Utc>) -> Vec<AddonInstanceView> {
        let guard = self.instances.lock().await;
        let cutoff =
            Duration::seconds(HEARTBEAT_INTERVAL_SECONDS * OFFLINE_AFTER_MISSED_HEARTBEATS);
        let mut views: Vec<AddonInstanceView> = guard
            .values()
            .map(|instance| AddonInstanceView {
                name: instance.name.clone(),
                version: instance.version.clone(),
                config_api_base_url: instance.config_api_base_url.clone(),
                online: now.signed_duration_since(instance.last_seen) <= cutoff,
            })
            .collect();
        views.sort_by(|a, b| a.name.cmp(&b.name));
        views
    }

    pub async fn find(&self, name: &str) -> Option<AddonInstance> {
        let guard = self.instances.lock().await;
        guard.get(name).cloned()
    }

    // Not called yet within this crate - Task 5 (persistence) wires these
    // into periodic save/load-on-startup. Silences dead_code until then.
    #[allow(dead_code)]
    pub(crate) async fn snapshot(&self) -> HashMap<String, AddonInstance> {
        self.instances.lock().await.clone()
    }

    #[allow(dead_code)]
    pub(crate) async fn replace_all(&self, instances: HashMap<String, AddonInstance>) {
        let mut guard = self.instances.lock().await;
        *guard = instances;
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(name: &str, last_seen: DateTime<Utc>) -> AddonInstance {
        AddonInstance {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            config_api_base_url: "http://10.0.0.32:8090".to_string(),
            last_seen,
        }
    }

    #[tokio::test]
    async fn registered_instance_is_online_immediately() {
        let registry = Registry::new();
        let now = Utc::now();
        registry.register(sample("kia-connect-bridge", now)).await;

        let views = registry.list(now).await;

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].name, "kia-connect-bridge");
        assert!(views[0].online);
    }

    #[tokio::test]
    async fn instance_goes_offline_after_missed_heartbeats() {
        let registry = Registry::new();
        let now = Utc::now();
        let stale = now - Duration::seconds(200); // > 180s cutoff
        registry.register(sample("kia-connect-bridge", stale)).await;

        let views = registry.list(now).await;

        assert!(!views[0].online);
    }

    #[tokio::test]
    async fn re_registering_same_name_is_last_write_wins() {
        let registry = Registry::new();
        let now = Utc::now();
        registry
            .register(sample("kia-connect-bridge", now - Duration::seconds(500)))
            .await;
        registry.register(sample("kia-connect-bridge", now)).await;

        let views = registry.list(now).await;

        assert_eq!(views.len(), 1);
        assert!(views[0].online);
    }

    #[tokio::test]
    async fn list_is_sorted_by_name() {
        let registry = Registry::new();
        let now = Utc::now();
        registry.register(sample("zzz-addon", now)).await;
        registry.register(sample("aaa-addon", now)).await;

        let views = registry.list(now).await;

        assert_eq!(views[0].name, "aaa-addon");
        assert_eq!(views[1].name, "zzz-addon");
    }
}
