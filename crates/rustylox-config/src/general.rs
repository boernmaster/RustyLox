//! General LoxBerry configuration (general.json)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::miniserver::MiniserverConfig;
use crate::mqtt::MqttConfig;

/// Top-level general configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeneralConfig {
    #[serde(rename = "Base")]
    pub base: BaseConfig,

    #[serde(rename = "Healthcheck")]
    pub healthcheck: Option<serde_json::Value>,

    #[serde(rename = "Miniserver")]
    pub miniserver: HashMap<String, MiniserverConfig>,

    #[serde(rename = "Backup")]
    pub backup: BackupConfig,

    #[serde(rename = "Mqtt")]
    pub mqtt: MqttConfig,

    #[serde(rename = "Network")]
    pub network: NetworkConfig,

    #[serde(rename = "Remote")]
    pub remote: RemoteConfig,

    #[serde(rename = "Ssdp")]
    pub ssdp: SsdpConfig,

    #[serde(rename = "Timeserver")]
    pub timeserver: TimeserverConfig,

    #[serde(rename = "Update")]
    pub update: UpdateConfig,

    #[serde(rename = "Watchdog")]
    pub watchdog: WatchdogConfig,

    #[serde(rename = "Webserver")]
    pub webserver: WebserverConfig,

    #[serde(rename = "Apt")]
    pub apt: AptConfig,

    #[serde(rename = "Weather", default)]
    pub weather: WeatherConfig,
}

/// Native weather service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConfig {
    /// Enable background weather fetching
    #[serde(rename = "Enabled", default)]
    pub enabled: bool,

    /// Decimal latitude (WGS84)
    #[serde(rename = "Latitude", default)]
    pub latitude: f64,

    /// Decimal longitude (WGS84)
    #[serde(rename = "Longitude", default)]
    pub longitude: f64,

    /// Human-readable location name shown in the UI
    #[serde(rename = "LocationName", default)]
    pub location_name: String,

    /// Elevation above sea level in metres (used for Loxone EMU)
    #[serde(rename = "Elevation", default)]
    pub elevation: f64,

    /// How often to refresh weather data (minutes, default 15)
    #[serde(
        rename = "UpdateIntervalMinutes",
        default = "WeatherConfig::default_interval"
    )]
    pub update_interval_minutes: u32,

    /// true = metric (°C, km/h, mm), false = imperial (°F, mph, in)
    #[serde(rename = "Metric", default = "WeatherConfig::default_true")]
    pub metric: bool,

    // ── Loxone UDP push ────────────────────────────────────────────────────
    /// Push weather values to Miniserver via UDP (same protocol as weather4lox)
    #[serde(rename = "PushUdp", default)]
    pub push_udp: bool,

    /// Which Miniserver entry (key in general.json) to push to
    #[serde(rename = "MiniserverKey", default = "WeatherConfig::default_ms_key")]
    pub miniserver_key: String,

    /// UDP port on the Miniserver that receives virtual inputs (default 7044)
    #[serde(
        rename = "MiniserverUdpPort",
        default = "WeatherConfig::default_udp_port"
    )]
    pub miniserver_udp_port: u16,

    // ── Loxone Cloud Emulator ──────────────────────────────────────────────
    /// Serve the Loxone weather.loxone.com API on port 6066 so the
    /// Miniserver can use RustyLox as its cloud weather source.
    #[serde(rename = "CloudEmu", default)]
    pub cloud_emu: bool,

    /// Write /etc/dnsmasq.d/rustylox-weather.conf to redirect
    /// weather.loxone.com → this host.  Requires dnsmasq + write permission.
    #[serde(rename = "DnsmasqEnabled", default)]
    pub dnsmasq_enabled: bool,

    /// Local IP advertised in the dnsmasq config (e.g. "192.168.1.10")
    #[serde(rename = "LocalIp", default)]
    pub local_ip: String,

    /// Resolved Miniserver IP for UDP push — not persisted, injected at runtime by the daemon.
    #[serde(skip)]
    pub miniserver_ip: String,

    // ── MQTT ───────────────────────────────────────────────────────────────
    /// Publish weather data to MQTT after each refresh
    #[serde(rename = "SendMqtt", default)]
    pub send_mqtt: bool,

    /// MQTT topic prefix (default "weather")
    #[serde(rename = "MqttTopic", default = "WeatherConfig::default_mqtt_topic")]
    pub mqtt_topic: String,
}

impl WeatherConfig {
    fn default_interval() -> u32 {
        15
    }
    fn default_true() -> bool {
        true
    }
    fn default_ms_key() -> String {
        "1".to_string()
    }
    fn default_udp_port() -> u16 {
        7044
    }
    fn default_mqtt_topic() -> String {
        "weather".to_string()
    }
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            latitude: 0.0,
            longitude: 0.0,
            location_name: String::new(),
            elevation: 0.0,
            update_interval_minutes: 15,
            metric: true,
            push_udp: false,
            miniserver_key: "1".to_string(),
            miniserver_udp_port: 7044,
            cloud_emu: false,
            dnsmasq_enabled: false,
            local_ip: String::new(),
            miniserver_ip: String::new(),
            send_mqtt: false,
            mqtt_topic: "weather".to_string(),
        }
    }
}

/// Base configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseConfig {
    #[serde(rename = "Clouddnsuri")]
    pub clouddnsuri: String,

    #[serde(rename = "Lang")]
    pub lang: String,

    #[serde(rename = "Sendstatistic")]
    pub sendstatistic: u8,

    #[serde(rename = "Startsetup")]
    pub startsetup: String,

    #[serde(rename = "Systemloglevel")]
    pub systemloglevel: String,

    #[serde(rename = "Version")]
    pub version: String,
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    #[serde(rename = "Keep_archives")]
    pub keep_archives: String,

    #[serde(rename = "Storagepath")]
    pub storagepath: String,

    #[serde(rename = "Compression")]
    pub compression: String,

    #[serde(rename = "Schedule")]
    pub schedule: BackupSchedule,
}

/// Backup schedule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSchedule {
    #[serde(rename = "Active")]
    pub active: String,

    /// Interval in hours between automatic backups
    #[serde(rename = "IntervalHours", default = "BackupSchedule::default_interval")]
    pub interval_hours: u64,

    /// Maximum number of automatic backups to keep
    #[serde(rename = "KeepBackups", default = "BackupSchedule::default_keep")]
    pub keep_backups: usize,

    /// Whether to include plugin data in scheduled backups
    #[serde(
        rename = "IncludePlugins",
        default = "BackupSchedule::default_include_plugins"
    )]
    pub include_plugins: bool,
}

impl BackupSchedule {
    fn default_interval() -> u64 {
        24
    }
    fn default_keep() -> usize {
        7
    }
    fn default_include_plugins() -> bool {
        true
    }
}

impl Default for BackupSchedule {
    fn default() -> Self {
        Self {
            active: "false".to_string(),
            interval_hours: 24,
            keep_backups: 7,
            include_plugins: true,
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    #[serde(rename = "Friendlyname")]
    pub friendlyname: Option<String>,

    #[serde(rename = "Interface")]
    pub interface: String,

    #[serde(rename = "Ipv4")]
    pub ipv4: Ipv4Config,

    #[serde(rename = "Ipv6")]
    pub ipv6: Ipv6Config,

    #[serde(rename = "Ssid")]
    pub ssid: String,

    #[serde(rename = "Wpa")]
    pub wpa: Option<String>,
}

/// IPv4 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ipv4Config {
    #[serde(rename = "Dns")]
    pub dns: String,

    #[serde(rename = "Gateway")]
    pub gateway: String,

    #[serde(rename = "Ipaddress")]
    pub ipaddress: String,

    #[serde(rename = "Mask")]
    pub mask: String,

    #[serde(rename = "Type")]
    pub type_: String,
}

/// IPv6 configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Ipv6Config {
    #[serde(rename = "Dns")]
    pub dns: Option<String>,

    #[serde(rename = "Ipaddress")]
    pub ipaddress: Option<String>,

    #[serde(rename = "Mask")]
    pub mask: Option<String>,

    #[serde(rename = "Privacyext")]
    pub privacyext: Option<String>,

    #[serde(rename = "Type")]
    pub type_: Option<String>,
}

/// Remote access configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    #[serde(rename = "Autoconnect")]
    pub autoconnect: String,

    #[serde(rename = "Httpport")]
    pub httpport: String,

    #[serde(rename = "Httpproxy")]
    pub httpproxy: String,
}

/// SSDP configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SsdpConfig {
    #[serde(rename = "Disabled")]
    pub disabled: Option<String>,

    #[serde(rename = "Uuid")]
    pub uuid: Option<String>,
}

/// Timeserver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeserverConfig {
    #[serde(rename = "Method")]
    pub method: String,

    #[serde(rename = "Ntpserver")]
    pub ntpserver: String,

    #[serde(rename = "Timemsno")]
    pub timemsno: u8,

    #[serde(rename = "Timezone")]
    pub timezone: String,
}

/// Update configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    #[serde(rename = "Branch")]
    pub branch: Option<String>,

    #[serde(rename = "Dryrun")]
    pub dryrun: Option<String>,

    #[serde(rename = "Failedscript")]
    pub failedscript: Option<String>,

    #[serde(rename = "Installtype")]
    pub installtype: String,

    #[serde(rename = "Interval")]
    pub interval: String,

    #[serde(rename = "Keepinstallfiles")]
    pub keepinstallfiles: Option<String>,

    #[serde(rename = "Keepupdatefiles")]
    pub keepupdatefiles: Option<String>,

    #[serde(rename = "Latestsha")]
    pub latestsha: Option<String>,

    #[serde(rename = "Releasetype")]
    pub releasetype: String,
}

/// Watchdog configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogConfig {
    #[serde(rename = "Maxtemp")]
    pub maxtemp: String,
}

/// Webserver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebserverConfig {
    #[serde(rename = "Port")]
    pub port: String,

    #[serde(rename = "Sslport")]
    pub sslport: String,

    #[serde(rename = "Sslenabled")]
    pub sslenabled: String,
}

/// APT configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AptConfig {
    #[serde(rename = "Servers")]
    pub servers: HashMap<String, String>,
}

impl Default for BaseConfig {
    fn default() -> Self {
        Self {
            clouddnsuri: "dns.loxonecloud.com".to_string(),
            lang: "en".to_string(),
            sendstatistic: 1,
            startsetup: "1".to_string(),
            systemloglevel: "6".to_string(),
            version: "4.0.0.0".to_string(), // Rust version
        }
    }
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            keep_archives: "1".to_string(),
            storagepath: String::new(),
            compression: "7z".to_string(),
            schedule: BackupSchedule::default(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            friendlyname: None,
            interface: "eth0".to_string(),
            ipv4: Ipv4Config::default(),
            ipv6: Ipv6Config::default(),
            ssid: String::new(),
            wpa: None,
        }
    }
}

impl Default for Ipv4Config {
    fn default() -> Self {
        Self {
            dns: String::new(),
            gateway: String::new(),
            ipaddress: String::new(),
            mask: String::new(),
            type_: "dhcp".to_string(),
        }
    }
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            autoconnect: "true".to_string(),
            httpport: String::new(),
            httpproxy: String::new(),
        }
    }
}

impl Default for TimeserverConfig {
    fn default() -> Self {
        Self {
            method: "ntp".to_string(),
            ntpserver: "0.europe.pool.ntp.org".to_string(),
            timemsno: 1,
            timezone: "Europe/Berlin".to_string(),
        }
    }
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            branch: None,
            dryrun: None,
            failedscript: None,
            installtype: "notify".to_string(),
            interval: "1".to_string(),
            keepinstallfiles: None,
            keepupdatefiles: None,
            latestsha: None,
            releasetype: "release".to_string(),
        }
    }
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            maxtemp: "85".to_string(),
        }
    }
}

impl Default for WebserverConfig {
    fn default() -> Self {
        Self {
            port: "80".to_string(),
            sslport: "443".to_string(),
            sslenabled: "false".to_string(),
        }
    }
}
