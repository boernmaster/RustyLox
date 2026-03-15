//! MQTT configuration

use serde::{Deserialize, Serialize};

/// MQTT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    #[serde(rename = "Brokerhost")]
    pub brokerhost: String,

    #[serde(rename = "Brokerport")]
    pub brokerport: String,

    #[serde(rename = "Brokeruser", default)]
    pub brokeruser: String,

    #[serde(rename = "Brokerpass", default)]
    pub brokerpass: String,

    #[serde(rename = "Udpinport")]
    pub udpinport: String,

    #[serde(rename = "Uselocalbroker")]
    pub uselocalbroker: String,

    #[serde(rename = "Websocketport")]
    pub websocketport: String,

    #[serde(rename = "Finderdisabled")]
    pub finderdisabled: bool,
}

impl MqttConfig {
    /// Get broker host (or default)
    pub fn broker_host(&self) -> &str {
        if self.brokerhost.is_empty() {
            "localhost"
        } else {
            &self.brokerhost
        }
    }

    /// Get broker port as u16
    pub fn broker_port(&self) -> u16 {
        self.brokerport.parse().unwrap_or(1883)
    }

    /// Get UDP input port as u16
    pub fn udp_port(&self) -> u16 {
        self.udpinport.parse().unwrap_or(11884)
    }

    /// Get WebSocket port as u16
    pub fn websocket_port(&self) -> u16 {
        self.websocketport.parse().unwrap_or(9001)
    }

    /// Check if using local broker
    pub fn uses_local_broker(&self) -> bool {
        self.uselocalbroker == "1"
    }
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            brokerhost: "localhost".to_string(),
            brokerport: "1883".to_string(),
            brokeruser: String::new(),
            brokerpass: String::new(),
            udpinport: "11884".to_string(),
            uselocalbroker: "1".to_string(),
            websocketport: "9001".to_string(),
            finderdisabled: false,
        }
    }
}
