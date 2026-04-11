//! Application state for the web API

use auth::AuthService;
use dashmap::DashMap;
use miniserver_client::MiniserverClient;
use rustylox_config::{ConfigManager, GeneralConfig};
use rustylox_metrics::collector::MetricsCollector;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};

use crate::weather::WeatherService;

/// Current active log level (stored as string for runtime mutation)
pub type LogLevelHandle = Arc<RwLock<String>>;

/// Miniserver communication event for monitoring
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MiniserverEvent {
    pub miniserver_id: u8,
    pub miniserver_name: String,
    pub direction: String, // "sent", "received", "error"
    pub protocol: String,  // "http", "udp"
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

    /// Build version (includes git tag/commit)
    pub version: String,

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

    /// Current log level (runtime-adjustable)
    pub log_level: LogLevelHandle,

    /// Authentication service (optional - disabled if not configured)
    pub auth_service: Option<Arc<AuthService>>,

    /// Native weather service (optional - only if enabled in config)
    pub weather_service: Option<Arc<WeatherService>>,

    /// Long-lived metrics collector — kept alive so sysinfo has a prior
    /// measurement interval and can return real CPU usage values.
    pub metrics_collector: Arc<Mutex<MetricsCollector>>,
}

impl AppState {
    /// Create new application state
    pub fn new(
        lbhomedir: PathBuf,
        version: String,
        config_manager: ConfigManager,
        config: GeneralConfig,
        mqtt_gateway: Option<Arc<mqtt_gateway::MqttGateway>>,
    ) -> Self {
        Self::new_with_shared_config(
            lbhomedir,
            version,
            config_manager,
            Arc::new(RwLock::new(config)),
            mqtt_gateway,
        )
    }

    /// Create new application state with a shared config (Arc<RwLock<GeneralConfig>>)
    pub fn new_with_shared_config(
        lbhomedir: PathBuf,
        version: String,
        config_manager: ConfigManager,
        config: Arc<RwLock<GeneralConfig>>,
        mqtt_gateway: Option<Arc<mqtt_gateway::MqttGateway>>,
    ) -> Self {
        // Create broadcast channel for monitoring (buffer 1000 events)
        let (monitor_tx, _) = broadcast::channel(1000);

        // Read initial log level from RUST_LOG env var
        let initial_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

        Self {
            lbhomedir,
            version,
            config_manager: Arc::new(config_manager),
            config,
            miniserver_clients: Arc::new(DashMap::new()),
            mqtt_gateway,
            miniserver_monitor: monitor_tx,
            log_level: Arc::new(RwLock::new(initial_level)),
            auth_service: None,
            weather_service: None,
            metrics_collector: Arc::new(Mutex::new(MetricsCollector::with_default_counters())),
        }
    }

    /// Attach an AuthService to the application state
    pub fn with_auth(mut self, auth_service: AuthService) -> Self {
        self.auth_service = Some(Arc::new(auth_service));
        self
    }

    /// Attach a WeatherService to the application state
    pub fn with_weather(mut self, weather_service: Arc<WeatherService>) -> Self {
        self.weather_service = Some(weather_service);
        self
    }

    /// Reload configuration from disk
    pub async fn reload_config(&self) -> rustylox_core::Result<()> {
        let new_config = self.config_manager.load_general().await?;
        let mut config = self.config.write().await;
        *config = new_config;
        Ok(())
    }

    /// Get a Miniserver client (creates if not exists)
    pub async fn get_miniserver_client(
        &self,
        id: u8,
    ) -> rustylox_core::Result<Arc<MiniserverClient>> {
        // Check if client exists
        if let Some(client) = self.miniserver_clients.get(&id) {
            return Ok(Arc::clone(client.value()));
        }

        // Create new client
        let config = self.config.read().await;
        let ms_config = config
            .miniserver
            .get(&id.to_string())
            .ok_or_else(|| rustylox_core::Error::config(format!("Miniserver {} not found", id)))?
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
                timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            });
        });

        client.http_mut().set_monitor_callback(callback);

        let client = Arc::new(client);
        self.miniserver_clients.insert(id, Arc::clone(&client));

        Ok(client)
    }
}
