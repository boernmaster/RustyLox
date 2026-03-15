//! Application state for the web API

use dashmap::DashMap;
use loxberry_config::{ConfigManager, GeneralConfig};
use miniserver_client::MiniserverClient;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Miniserver communication event for monitoring
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MiniserverEvent {
    pub miniserver_id: u8,
    pub miniserver_name: String,
    pub direction: String,  // "sent", "received", "error"
    pub protocol: String,   // "http", "udp"
    pub url: Option<String>,
    pub params: Option<String>,
    pub response: Option<String>,
    pub code: Option<String>,
    pub error: Option<String>,
    pub timestamp: String,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// LoxBerry home directory
    pub lbhomedir: PathBuf,

    /// Configuration manager
    pub config_manager: Arc<ConfigManager>,

    /// Current configuration (cached)
    pub config: Arc<RwLock<GeneralConfig>>,

    /// Miniserver clients (by ID) - wrapped in Arc for sharing
    pub miniserver_clients: Arc<DashMap<u8, Arc<MiniserverClient>>>,

    /// MQTT Gateway (optional - only if enabled)
    pub mqtt_gateway: Option<Arc<mqtt_gateway::MqttGateway>>,

    /// Broadcast channel for Miniserver monitoring events
    pub miniserver_monitor: broadcast::Sender<MiniserverEvent>,
}

impl AppState {
    /// Create new application state
    pub fn new(
        lbhomedir: PathBuf,
        config_manager: ConfigManager,
        config: GeneralConfig,
        mqtt_gateway: Option<Arc<mqtt_gateway::MqttGateway>>,
    ) -> Self {
        // Create broadcast channel for monitoring (buffer 1000 events)
        let (monitor_tx, _) = broadcast::channel(1000);

        Self {
            lbhomedir,
            config_manager: Arc::new(config_manager),
            config: Arc::new(RwLock::new(config)),
            miniserver_clients: Arc::new(DashMap::new()),
            mqtt_gateway,
            miniserver_monitor: monitor_tx,
        }
    }

    /// Reload configuration from disk
    pub async fn reload_config(&self) -> loxberry_core::Result<()> {
        let new_config = self.config_manager.load_general().await?;
        let mut config = self.config.write().await;
        *config = new_config;
        Ok(())
    }

    /// Get a Miniserver client (creates if not exists)
    pub async fn get_miniserver_client(
        &self,
        id: u8,
    ) -> loxberry_core::Result<Arc<MiniserverClient>> {
        // Check if client exists
        if let Some(client) = self.miniserver_clients.get(&id) {
            return Ok(Arc::clone(client.value()));
        }

        // Create new client
        let config = self.config.read().await;
        let ms_config = config
            .miniserver
            .get(&id.to_string())
            .ok_or_else(|| loxberry_core::Error::config(format!("Miniserver {} not found", id)))?
            .clone();

        let miniserver_name = ms_config.name.clone();
        let mut client = MiniserverClient::new(ms_config)?;

        // Set up monitoring callback
        let monitor_tx = self.miniserver_monitor.clone();
        let callback: miniserver_client::MonitorCallback = Arc::new(move |event| {
            let _ = monitor_tx.send(MiniserverEvent {
                miniserver_id: id,
                miniserver_name: miniserver_name.clone(),
                direction: event.direction,
                protocol: event.protocol,
                url: event.url,
                params: event.params,
                response: event.response,
                code: event.code,
                error: event.error,
                timestamp: chrono::Utc::now()
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
            });
        });

        client.http_mut().set_monitor_callback(callback);

        let client = Arc::new(client);
        self.miniserver_clients.insert(id, Arc::clone(&client));

        Ok(client)
    }
}
