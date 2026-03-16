//! Configuration validation for LoxBerry

use crate::{MiniserverConfig, MqttConfig};
use loxberry_core::{Error, Result};

/// Validate Miniserver configuration
pub fn validate_miniserver_config(config: &MiniserverConfig) -> Result<()> {
    if config.name.is_empty() {
        return Err(Error::config("Miniserver name is required"));
    }

    if config.ipaddress.is_empty() {
        return Err(Error::config("Miniserver IP address is required"));
    }

    let port: u16 = config
        .port
        .parse()
        .map_err(|_| Error::config("Miniserver port must be a valid number (1-65535)"))?;

    if port == 0 {
        return Err(Error::config("Miniserver port must be between 1 and 65535"));
    }

    if let Some(ref https_port) = config.porthttps {
        if !https_port.is_empty() {
            let p: u16 = https_port
                .parse()
                .map_err(|_| Error::config("HTTPS port must be a valid number (1-65535)"))?;
            if p == 0 {
                return Err(Error::config("HTTPS port must be between 1 and 65535"));
            }
        }
    }

    if !["http", "https"].contains(&config.transport.as_str()) {
        return Err(Error::config(
            "Miniserver transport must be 'http' or 'https'",
        ));
    }

    Ok(())
}

/// Validate MQTT configuration
pub fn validate_mqtt_config(config: &MqttConfig) -> Result<()> {
    if config.brokerhost.is_empty() {
        return Err(Error::config("MQTT broker host is required"));
    }

    let port: u16 = config
        .brokerport
        .parse()
        .map_err(|_| Error::config("MQTT broker port must be a valid number (1-65535)"))?;

    if port == 0 {
        return Err(Error::config("MQTT broker port must be between 1 and 65535"));
    }

    let udp_port: u16 = config
        .udpinport
        .parse()
        .map_err(|_| Error::config("UDP input port must be a valid number (1-65535)"))?;

    if udp_port == 0 {
        return Err(Error::config("UDP input port must be between 1 and 65535"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_miniserver() {
        let config = MiniserverConfig {
            name: "MS1".to_string(),
            ipaddress: "192.168.1.100".to_string(),
            port: "80".to_string(),
            transport: "http".to_string(),
            ..Default::default()
        };
        assert!(validate_miniserver_config(&config).is_ok());
    }

    #[test]
    fn test_miniserver_empty_name() {
        let config = MiniserverConfig {
            name: String::new(),
            ipaddress: "192.168.1.100".to_string(),
            port: "80".to_string(),
            transport: "http".to_string(),
            ..Default::default()
        };
        assert!(validate_miniserver_config(&config).is_err());
    }

    #[test]
    fn test_miniserver_invalid_port() {
        let config = MiniserverConfig {
            name: "MS1".to_string(),
            ipaddress: "192.168.1.100".to_string(),
            port: "not_a_port".to_string(),
            transport: "http".to_string(),
            ..Default::default()
        };
        assert!(validate_miniserver_config(&config).is_err());
    }

    #[test]
    fn test_valid_mqtt() {
        let config = MqttConfig::default();
        assert!(validate_mqtt_config(&config).is_ok());
    }

    #[test]
    fn test_mqtt_empty_host() {
        let config = MqttConfig {
            brokerhost: String::new(),
            ..Default::default()
        };
        assert!(validate_mqtt_config(&config).is_err());
    }
}
