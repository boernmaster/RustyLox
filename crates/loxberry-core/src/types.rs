//! Common types for LoxBerry

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Miniserver identifier (1-indexed)
pub type MiniserverNumber = u8;

/// Plugin MD5 hash identifier
pub type PluginMd5 = String;

/// Plugin folder name
pub type PluginFolder = String;

/// System paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoxBerryPaths {
    /// LoxBerry home directory (typically /opt/loxberry)
    pub lbhomedir: String,

    /// System HTML directory
    pub lbshtmldir: String,

    /// System HTML auth directory
    pub lbshtmlauthdir: String,

    /// System template directory
    pub lbstemplatedir: String,

    /// System data directory
    pub lbsdatadir: String,

    /// System log directory
    pub lbslogdir: String,

    /// System tmpfs log directory
    pub lbstmpfslogdir: String,

    /// System config directory
    pub lbsconfigdir: String,

    /// System sbin directory
    pub lbssbindir: String,

    /// System bin directory
    pub lbsbindir: String,
}

impl Default for LoxBerryPaths {
    fn default() -> Self {
        let lbhomedir = "/opt/loxberry".to_string();
        Self {
            lbhomedir: lbhomedir.clone(),
            lbshtmldir: format!("{}/webfrontend/html/system", lbhomedir),
            lbshtmlauthdir: format!("{}/webfrontend/htmlauth/system", lbhomedir),
            lbstemplatedir: format!("{}/templates/system", lbhomedir),
            lbsdatadir: format!("{}/data/system", lbhomedir),
            lbslogdir: format!("{}/log/system", lbhomedir),
            lbstmpfslogdir: format!("{}/log/system_tmpfs", lbhomedir),
            lbsconfigdir: format!("{}/config/system", lbhomedir),
            lbssbindir: format!("{}/sbin", lbhomedir),
            lbsbindir: format!("{}/bin", lbhomedir),
        }
    }
}

/// Plugin-specific paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPaths {
    /// Plugin folder name
    pub plugin_folder: String,

    /// Plugin HTML auth directory
    pub lbphtmlauthdir: String,

    /// Plugin HTML directory
    pub lbphtmldir: String,

    /// Plugin template directory
    pub lbptemplatedir: String,

    /// Plugin data directory
    pub lbpdatadir: String,

    /// Plugin log directory
    pub lbplogdir: String,

    /// Plugin config directory
    pub lbpconfigdir: String,

    /// Plugin bin directory
    pub lbpbindir: String,
}

impl PluginPaths {
    /// Create plugin paths for a given plugin folder
    pub fn new(lbhomedir: &str, plugin_folder: impl Into<String>) -> Self {
        let plugin_folder = plugin_folder.into();
        Self {
            plugin_folder: plugin_folder.clone(),
            lbphtmlauthdir: format!("{}/webfrontend/htmlauth/plugins/{}", lbhomedir, plugin_folder),
            lbphtmldir: format!("{}/webfrontend/html/plugins/{}", lbhomedir, plugin_folder),
            lbptemplatedir: format!("{}/templates/plugins/{}", lbhomedir, plugin_folder),
            lbpdatadir: format!("{}/data/plugins/{}", lbhomedir, plugin_folder),
            lbplogdir: format!("{}/log/plugins/{}", lbhomedir, plugin_folder),
            lbpconfigdir: format!("{}/config/plugins/{}", lbhomedir, plugin_folder),
            lbpbindir: format!("{}/bin/plugins/{}", lbhomedir, plugin_folder),
        }
    }

    /// Convert to environment variables for plugin execution
    pub fn to_env_vars(&self, lbhomedir: &str) -> HashMap<String, String> {
        let mut env = HashMap::new();
        env.insert("LBHOMEDIR".to_string(), lbhomedir.to_string());
        env.insert("LBPPLUGINDIR".to_string(), self.plugin_folder.clone());
        env.insert("LBPHTMLAUTHDIR".to_string(), self.lbphtmlauthdir.clone());
        env.insert("LBPHTMLDIR".to_string(), self.lbphtmldir.clone());
        env.insert("LBPTEMPLATEDIR".to_string(), self.lbptemplatedir.clone());
        env.insert("LBPDATADIR".to_string(), self.lbpdatadir.clone());
        env.insert("LBPLOGDIR".to_string(), self.lbplogdir.clone());
        env.insert("LBPCONFIGDIR".to_string(), self.lbpconfigdir.clone());
        env.insert("LBPBINDIR".to_string(), self.lbpbindir.clone());
        env
    }
}
