//! Askama templates

use askama::Template;
use serde::{Deserialize, Serialize};

/// Base layout template
#[derive(Template)]
#[template(path = "base.html")]
pub struct BaseTemplate<'a> {
    pub title: &'a str,
    pub content: String,
    pub version: String,
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
    pub version: String,
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

/// Miniserver Monitor template
#[derive(Template)]
#[template(path = "miniserver/monitor.html")]
pub struct MiniserverMonitorTemplate {
    pub title: String,
    pub version: String,
}

/// Miniserver communication message for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiniserverMessage {
    pub direction: String, // "sent", "received", "error"
    pub protocol: String,  // "http", "udp"
    pub miniserver_name: String,
    pub url: Option<String>,
    pub params: Option<String>,
    pub response: Option<String>,
    pub code: Option<String>,
    pub error: Option<String>,
    pub timestamp: String,
}

/// Miniserver list template
#[derive(Template)]
#[template(path = "miniserver/list.html")]
pub struct MiniserverListTemplate {
    pub miniservers: Vec<MiniserverDisplay>,
    pub version: String,
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
    pub version: String,
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
    pub version: String,
}

/// Plugin details template
#[derive(Template)]
#[template(path = "plugins/details.html")]
pub struct PluginDetailsTemplate {
    pub plugin: PluginDisplay,
    pub version: String,
}

/// Plugin install template
#[derive(Template)]
#[template(path = "plugins/install.html")]
pub struct PluginInstallTemplate {
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDisplay {
    pub md5: String,
    pub name: String,
    pub folder: String,
    pub version: String,
    pub author: String,
    pub author_email: String,
    pub title: String,
    pub has_web_ui: bool,
    pub has_daemon: bool,
    pub daemon_running: bool,
    pub install_date: String,
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
    pub topicfilter: String,
}

/// Settings template
#[derive(Template)]
#[template(path = "settings.html")]
pub struct SettingsTemplate {
    pub language: String,
    pub timezone: String,
    pub version: String,
}

/// Email configuration template
#[derive(Template)]
#[template(path = "email.html")]
pub struct EmailTemplate {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub smtp_tls: bool,
    pub from_address: String,
    pub from_name: String,
    pub notification_addresses: String,
    pub enabled: bool,
    pub version: String,
}

/// Scheduled tasks template
#[derive(Template)]
#[template(path = "tasks.html")]
pub struct TasksTemplate {
    pub version: String,
}

/// Network diagnostics template
#[derive(Template)]
#[template(path = "network.html")]
pub struct NetworkTemplate {
    pub version: String,
}

/// Login page template
#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
    pub version: String,
}

/// Admin user management template
#[derive(Template)]
#[template(path = "admin/users.html")]
pub struct AdminUsersTemplate {
    pub version: String,
}

/// Admin API keys template
#[derive(Template)]
#[template(path = "admin/api_keys.html")]
pub struct AdminApiKeysTemplate {
    pub version: String,
}

/// Admin audit log template
#[derive(Template)]
#[template(path = "admin/audit.html")]
pub struct AdminAuditTemplate {
    pub version: String,
}

/// API documentation template
#[derive(Template)]
#[template(path = "api_docs.html")]
pub struct ApiDocsTemplate {
    pub groups: Vec<ApiGroup>,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct ApiGroup {
    pub name: String,
    pub endpoints: Vec<ApiEndpoint>,
}

#[derive(Debug, Clone)]
pub struct ApiEndpoint {
    pub method: String,
    pub path: String,
    pub description: String,
}
