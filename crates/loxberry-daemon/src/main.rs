//! LoxBerry Daemon - Main entry point
//!
//! This is the main orchestrator binary that starts all services.

use anyhow::Result;
use loxberry_config::{ConfigManager, GeneralConfig};
use std::path::PathBuf;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use web_api::{create_router, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "loxberry_daemon=info,web_api=info,miniserver_client=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting LoxBerry Daemon v{}", env!("CARGO_PKG_VERSION"));

    // Get configuration directory from environment or use default
    let config_dir = std::env::var("LBHOMEDIR")
        .map(|dir| PathBuf::from(dir).join("config/system"))
        .unwrap_or_else(|_| PathBuf::from("/opt/loxberry/config/system"));

    info!("Using configuration directory: {}", config_dir.display());

    // Create configuration manager
    let config_manager = ConfigManager::new(&config_dir);

    // Load configuration (or create default if not exists)
    let config = match config_manager.load_general().await {
        Ok(cfg) => {
            info!("Loaded configuration from {}", config_manager.general_json_path().display());
            cfg
        }
        Err(e) => {
            warn!("Failed to load configuration: {}", e);
            warn!("Using default configuration");
            GeneralConfig::default()
        }
    };

    // Create application state
    let state = AppState::new(config_manager, config);

    // Create router
    let app = create_router(state);

    // Get bind address from environment or use default
    let bind_addr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    info!("Starting web server on http://{}", bind_addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    info!("LoxBerry Daemon is running!");
    info!("API available at: http://{}", bind_addr);
    info!("Health check: http://{}/health", bind_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
