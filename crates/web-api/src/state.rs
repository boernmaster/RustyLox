//! Application state for the web API

use dashmap::DashMap;
use loxberry_config::{ConfigManager, GeneralConfig};
use miniserver_client::MiniserverClient;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

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
}

impl AppState {
    /// Create new application state
    pub fn new(lbhomedir: PathBuf, config_manager: ConfigManager, config: GeneralConfig) -> Self {
        Self {
            lbhomedir,
            config_manager: Arc::new(config_manager),
            config: Arc::new(RwLock::new(config)),
            miniserver_clients: Arc::new(DashMap::new()),
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
    pub async fn get_miniserver_client(&self, id: u8) -> loxberry_core::Result<Arc<MiniserverClient>> {
        // Check if client exists
        if let Some(client) = self.miniserver_clients.get(&id) {
            return Ok(Arc::clone(client.value()));
        }

        // Create new client
        let config = self.config.read().await;
        let ms_config = config
            .miniserver
            .get(&id.to_string())
            .ok_or_else(|| {
                loxberry_core::Error::config(format!("Miniserver {} not found", id))
            })?
            .clone();

        let client = Arc::new(MiniserverClient::new(ms_config)?);
        self.miniserver_clients.insert(id, Arc::clone(&client));

        Ok(client)
    }
}
