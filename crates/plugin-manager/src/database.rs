//! Plugin database management
//!
//! Manages plugindatabase.json which stores all installed plugins

use loxberry_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tracing::{debug, info};

/// Plugin database stored in JSON format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDatabase {
    /// Map of MD5 hash to plugin entry
    pub plugins: HashMap<String, PluginEntry>,
}

/// Plugin entry in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    /// MD5 checksum (author_name + author_email + name + folder)
    pub md5: String,

    /// Author name
    pub author_name: String,

    /// Author email
    pub author_email: String,

    /// Plugin version
    pub version: String,

    /// Plugin name (unique identifier)
    pub name: String,

    /// Plugin folder name
    pub folder: String,

    /// Plugin title (multilingual)
    pub title: HashMap<String, String>,

    /// Web interface path (relative to htmlauth)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<String>,

    /// Automatic updates setting (0=n/a, 1=disabled, 2=notify, 3=release, 4=prerelease)
    #[serde(default)]
    pub autoupdate: u8,

    /// Release configuration URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub releasecfg: Option<String>,

    /// Prerelease configuration URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prereleasecfg: Option<String>,

    /// Log level (0-7)
    #[serde(default = "default_loglevel")]
    pub loglevel: String,

    /// Plugin directories
    pub directories: PluginDirectories,

    /// Installation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_timestamp: Option<u64>,

    /// Last update timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_timestamp: Option<u64>,
}

/// Plugin directory paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDirectories {
    pub htmlauth: String,
    pub html: String,
    pub template: String,
    pub data: String,
    pub log: String,
    pub config: String,
    pub bin: String,
}

fn default_loglevel() -> String {
    "6".to_string()
}

impl PluginDatabase {
    /// Load plugin database from file
    pub async fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            info!(
                "Plugin database not found, creating new one: {}",
                path.display()
            );
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path)
            .await
            .map_err(|e| Error::plugin(format!("Failed to read plugin database: {}", e)))?;

        let db: PluginDatabase = serde_json::from_str(&content)
            .map_err(|e| Error::plugin(format!("Failed to parse plugin database: {}", e)))?;

        debug!("Loaded {} plugins from database", db.plugins.len());
        Ok(db)
    }

    /// Save plugin database to file
    pub async fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                Error::plugin(format!("Failed to create database directory: {}", e))
            })?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| Error::plugin(format!("Failed to serialize plugin database: {}", e)))?;

        fs::write(path, content)
            .await
            .map_err(|e| Error::plugin(format!("Failed to write plugin database: {}", e)))?;

        debug!("Saved {} plugins to database", self.plugins.len());
        Ok(())
    }

    /// Create a new empty database
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Find plugin by MD5 hash
    pub fn find_by_md5(&self, md5: &str) -> Option<&PluginEntry> {
        self.plugins.get(md5)
    }

    /// Find plugin by folder name
    pub fn find_by_folder(&self, folder: &str) -> Option<&PluginEntry> {
        self.plugins.values().find(|p| p.folder == folder)
    }

    /// Find plugin by name
    pub fn find_by_name(&self, name: &str) -> Option<&PluginEntry> {
        self.plugins.values().find(|p| p.name == name)
    }

    /// Add or update plugin
    pub fn upsert(&mut self, plugin: PluginEntry) {
        let md5 = plugin.md5.clone();
        self.plugins.insert(md5, plugin);
    }

    /// Remove plugin by MD5
    pub fn remove(&mut self, md5: &str) -> Option<PluginEntry> {
        self.plugins.remove(md5)
    }

    /// List all plugins
    pub fn list(&self) -> Vec<&PluginEntry> {
        self.plugins.values().collect()
    }

    /// Count plugins
    pub fn count(&self) -> usize {
        self.plugins.len()
    }
}

impl Default for PluginDatabase {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate MD5 checksum for plugin identification
///
/// Format: MD5(author_name + author_email + name + folder)
pub fn calculate_plugin_md5(
    author_name: &str,
    author_email: &str,
    name: &str,
    folder: &str,
) -> String {
    let combined = format!("{}{}{}{}", author_name, author_email, name, folder);
    let digest = md5::compute(combined.as_bytes());
    format!("{:x}", digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_md5() {
        let md5 = calculate_plugin_md5("John Doe", "john@example.com", "TestPlugin", "testplugin");

        // MD5 should be 32 hex characters
        assert_eq!(md5.len(), 32);

        // Same input should produce same output
        let md5_2 =
            calculate_plugin_md5("John Doe", "john@example.com", "TestPlugin", "testplugin");
        assert_eq!(md5, md5_2);
    }

    #[test]
    fn test_database_operations() {
        let mut db = PluginDatabase::new();

        let plugin = PluginEntry {
            md5: "abc123".to_string(),
            author_name: "Test Author".to_string(),
            author_email: "test@example.com".to_string(),
            version: "1.0.0".to_string(),
            name: "TestPlugin".to_string(),
            folder: "testplugin".to_string(),
            title: [("en".to_string(), "Test Plugin".to_string())]
                .iter()
                .cloned()
                .collect(),
            interface: Some("index.html".to_string()),
            autoupdate: 0,
            releasecfg: None,
            prereleasecfg: None,
            loglevel: "6".to_string(),
            directories: PluginDirectories {
                htmlauth: "/opt/loxberry/webfrontend/htmlauth/plugins/testplugin".to_string(),
                html: "/opt/loxberry/webfrontend/html/plugins/testplugin".to_string(),
                template: "/opt/loxberry/templates/plugins/testplugin".to_string(),
                data: "/opt/loxberry/data/plugins/testplugin".to_string(),
                log: "/opt/loxberry/log/plugins/testplugin".to_string(),
                config: "/opt/loxberry/config/plugins/testplugin".to_string(),
                bin: "/opt/loxberry/bin/plugins/testplugin".to_string(),
            },
            install_timestamp: None,
            update_timestamp: None,
        };

        // Test upsert
        db.upsert(plugin.clone());
        assert_eq!(db.count(), 1);

        // Test find by MD5
        assert!(db.find_by_md5("abc123").is_some());
        assert!(db.find_by_md5("nonexistent").is_none());

        // Test find by folder
        assert!(db.find_by_folder("testplugin").is_some());

        // Test remove
        let removed = db.remove("abc123");
        assert!(removed.is_some());
        assert_eq!(db.count(), 0);
    }
}
