//! Askama templates

use askama::Template;
use serde::{Deserialize, Serialize};

/// Base layout template
#[derive(Template)]
#[template(path = "base.html")]
pub struct BaseTemplate<'a> {
    pub title: &'a str,
    pub content: String,
}

/// Dashboard template
#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub system_status: SystemStatus,
    pub miniserver_count: usize,
    pub plugin_count: usize,
    pub mqtt_connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub version: String,
    pub status: String,
    pub uptime: String,
}

/// MQTT Monitor template
#[derive(Template)]
#[template(path = "mqtt/monitor.html")]
pub struct MqttMonitorTemplate {
    pub title: String,
}

/// MQTT Message for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: String,
    pub qos: u8,
    pub retain: bool,
    pub timestamp: String,
}

/// Miniserver list template
#[derive(Template)]
#[template(path = "miniserver/list.html")]
pub struct MiniserverListTemplate {
    pub miniservers: Vec<MiniserverDisplay>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiniserverDisplay {
    pub id: String,
    pub name: String,
    pub ipaddress: String,
    pub port: String,
    pub connected: bool,
}

/// Miniserver edit template
#[derive(Template)]
#[template(path = "miniserver/edit.html")]
pub struct MiniserverEditTemplate {
    pub miniserver: Option<MiniserverForm>,
    pub is_new: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiniserverForm {
    pub id: Option<String>,
    pub name: String,
    pub ipaddress: String,
    pub port: String,
    pub admin: String,
    pub pass: String,
    pub useclouddns: bool,
}

/// Plugin list template
#[derive(Template)]
#[template(path = "plugins/list.html")]
pub struct PluginListTemplate {
    pub plugins: Vec<PluginDisplay>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDisplay {
    pub md5: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub title: String,
}

/// MQTT config template
#[derive(Template)]
#[template(path = "mqtt/config.html")]
pub struct MqttConfigTemplate {
    pub config: MqttConfigForm,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfigForm {
    pub brokerhost: String,
    pub brokerport: String,
    pub brokeruser: String,
    pub brokerpass: String,
    pub udpinport: String,
}

/// Settings template
#[derive(Template)]
#[template(path = "settings.html")]
pub struct SettingsTemplate {
    pub language: String,
    pub timezone: String,
    pub version: String,
}
