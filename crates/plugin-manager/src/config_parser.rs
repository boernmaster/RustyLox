//! Plugin configuration parser
//!
//! Parses plugin.cfg files in INI format

use loxberry_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::debug;

/// Plugin configuration from plugin.cfg
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub author: AuthorInfo,
    pub plugin: PluginInfo,
    pub system: Option<SystemRequirements>,
    pub daemon: Option<DaemonConfig>,
    pub cron: Option<CronConfig>,
    pub sudoers: Option<SudoersConfig>,
    pub apt: Option<AptConfig>,
}

/// Author information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorInfo {
    pub name: String,
    pub email: String,
}

/// Plugin information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub folder: String,
    pub version: String,
    pub title: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autoupdate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub releasecfg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prereleasecfg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loglevel: Option<String>,
}

/// System requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemRequirements {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lb_minimum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lb_maximum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
}

/// Daemon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub daemon: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<String>,
}

/// Cron configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronConfig {
    pub schedule: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<String>,
}

/// Sudoers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SudoersConfig {
    pub commands: Vec<String>,
}

/// APT packages configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AptConfig {
    pub packages: Vec<String>,
}

impl PluginConfig {
    /// Parse plugin.cfg from INI file
    pub fn parse(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            Error::plugin(format!("Failed to read plugin.cfg: {}", e))
        })?;

        Self::parse_from_str(&content)
    }

    /// Parse plugin.cfg from string content
    pub fn parse_from_str(content: &str) -> Result<Self> {
        // Parse INI format manually since serde_ini doesn't handle our complex structure well
        let ini = parse_ini(content)?;

        // Extract author info
        let author_section = ini.get("AUTHOR")
            .ok_or_else(|| Error::plugin("Missing [AUTHOR] section in plugin.cfg"))?;

        let author = AuthorInfo {
            name: author_section.get("NAME")
                .ok_or_else(|| Error::plugin("Missing AUTHOR.NAME in plugin.cfg"))?
                .clone(),
            email: author_section.get("EMAIL")
                .ok_or_else(|| Error::plugin("Missing AUTHOR.EMAIL in plugin.cfg"))?
                .clone(),
        };

        // Extract plugin info
        let plugin_section = ini.get("PLUGIN")
            .ok_or_else(|| Error::plugin("Missing [PLUGIN] section in plugin.cfg"))?;

        let name = plugin_section.get("NAME")
            .ok_or_else(|| Error::plugin("Missing PLUGIN.NAME in plugin.cfg"))?
            .clone();

        let folder = plugin_section.get("FOLDER")
            .ok_or_else(|| Error::plugin("Missing PLUGIN.FOLDER in plugin.cfg"))?
            .clone();

        let version = plugin_section.get("VERSION")
            .ok_or_else(|| Error::plugin("Missing PLUGIN.VERSION in plugin.cfg"))?
            .clone();

        // Extract titles (TITLE_EN, TITLE_DE, etc.)
        let mut title = HashMap::new();
        for (key, value) in plugin_section {
            if key.starts_with("TITLE_") {
                let lang = key.strip_prefix("TITLE_").unwrap().to_lowercase();
                title.insert(lang, value.clone());
            }
        }

        let plugin = PluginInfo {
            name,
            folder,
            version,
            title,
            interface: plugin_section.get("INTERFACE").cloned(),
            autoupdate: plugin_section.get("AUTOUPDATE").cloned(),
            releasecfg: plugin_section.get("RELEASECFG").cloned(),
            prereleasecfg: plugin_section.get("PRERELEASECFG").cloned(),
            loglevel: plugin_section.get("LOGLEVEL").cloned(),
        };

        // Extract system requirements (optional)
        let system = ini.get("SYSTEM").map(|sys| SystemRequirements {
            lb_minimum: sys.get("LB_MINIMUM").cloned(),
            lb_maximum: sys.get("LB_MAXIMUM").cloned(),
            architecture: sys.get("ARCHITECTURE").cloned(),
            os: sys.get("OS").cloned(),
        });

        // Extract daemon config (optional)
        let daemon = ini.get("DAEMON").map(|d| DaemonConfig {
            daemon: d.get("DAEMON").cloned().unwrap_or_default(),
            enabled: d.get("ENABLED").cloned(),
        });

        // Extract cron config (optional)
        let cron = ini.get("CRON").map(|c| CronConfig {
            schedule: c.get("SCHEDULE").cloned().unwrap_or_default(),
            command: c.get("COMMAND").cloned().unwrap_or_default(),
            enabled: c.get("ENABLED").cloned(),
        });

        // Extract sudoers config (optional)
        let sudoers = ini.get("SUDOERS").map(|s| {
            let commands: Vec<String> = s.iter()
                .filter(|(k, _)| k.starts_with("COMMAND"))
                .map(|(_, v)| v.clone())
                .collect();
            SudoersConfig { commands }
        });

        // Extract APT packages (optional)
        let apt = ini.get("APT").map(|a| {
            let packages: Vec<String> = a.iter()
                .filter(|(k, _)| k.starts_with("PACKAGE"))
                .map(|(_, v)| v.clone())
                .collect();
            AptConfig { packages }
        });

        Ok(PluginConfig {
            author,
            plugin,
            system,
            daemon,
            cron,
            sudoers,
            apt,
        })
    }
}

/// Simple INI parser
fn parse_ini(content: &str) -> Result<HashMap<String, HashMap<String, String>>> {
    let mut result: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_section: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        // Check for section header
        if line.starts_with('[') && line.ends_with(']') {
            let section = line[1..line.len() - 1].to_string();
            current_section = Some(section.clone());
            result.insert(section, HashMap::new());
            continue;
        }

        // Parse key=value
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_uppercase();
            let value = value.trim().to_string();

            if let Some(ref section_name) = current_section {
                if let Some(section) = result.get_mut(section_name) {
                    section.insert(key, value);
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plugin_cfg() {
        let cfg = r#"
[AUTHOR]
NAME=John Doe
EMAIL=john@example.com

[PLUGIN]
NAME=TestPlugin
FOLDER=testplugin
VERSION=1.0.0
TITLE_EN=Test Plugin
TITLE_DE=Test Plugin DE
INTERFACE=index.html
AUTOUPDATE=1
LOGLEVEL=6

[SYSTEM]
LB_MINIMUM=3.0.0
LB_MAXIMUM=4.0.0

[DAEMON]
DAEMON=/opt/loxberry/bin/plugins/testplugin/daemon.pl
ENABLED=1
"#;

        let config = PluginConfig::parse_from_str(cfg).unwrap();

        assert_eq!(config.author.name, "John Doe");
        assert_eq!(config.author.email, "john@example.com");
        assert_eq!(config.plugin.name, "TestPlugin");
        assert_eq!(config.plugin.folder, "testplugin");
        assert_eq!(config.plugin.version, "1.0.0");
        assert_eq!(config.plugin.title.get("en"), Some(&"Test Plugin".to_string()));
        assert!(config.system.is_some());
        assert!(config.daemon.is_some());
    }
}
