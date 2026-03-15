//! Delta cache for optimized parameter sending
//!
//! Caches previously sent values to avoid redundant network traffic.

use dashmap::DashMap;
use std::sync::Arc;

/// Thread-safe delta cache for parameter values
#[derive(Debug, Clone)]
pub struct DeltaCache {
    cache: Arc<DashMap<String, String>>,
}

impl DeltaCache {
    /// Create a new delta cache
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
        }
    }

    /// Check if a parameter value has changed from cached value
    ///
    /// Returns true if the value is different or not in cache
    pub fn has_changed(&self, key: &str, value: &str) -> bool {
        match self.cache.get(key) {
            Some(cached) => cached.value() != value,
            None => true, // Not in cache, treat as changed
        }
    }

    /// Update cached value for a parameter
    pub fn update(&self, key: &str, value: String) {
        self.cache.insert(key.to_string(), value);
    }

    /// Get cached value for a parameter
    pub fn get(&self, key: &str) -> Option<String> {
        self.cache.get(key).map(|v| v.value().clone())
    }

    /// Clear all cached values
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Get number of cached entries
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for DeltaCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_cache() {
        let cache = DeltaCache::new();

        // First check - value not in cache, should be treated as changed
        assert!(cache.has_changed("temp", "23.5"));

        // Update cache
        cache.update("temp", "23.5".to_string());

        // Same value - should not be changed
        assert!(!cache.has_changed("temp", "23.5"));

        // Different value - should be changed
        assert!(cache.has_changed("temp", "24.0"));

        // Update with new value
        cache.update("temp", "24.0".to_string());

        // Verify update
        assert_eq!(cache.get("temp"), Some("24.0".to_string()));
        assert!(!cache.has_changed("temp", "24.0"));
    }

    #[test]
    fn test_clear() {
        let cache = DeltaCache::new();

        cache.update("key1", "value1".to_string());
        cache.update("key2", "value2".to_string());

        assert_eq!(cache.len(), 2);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }
}
