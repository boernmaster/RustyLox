//! LoxBerry Daemon - Main entry point
//!
//! This is the main orchestrator binary that starts all services.

use anyhow::Result;
use loxberry_config::{ConfigManager, GeneralConfig};
use mqtt_gateway::MqttGateway;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn, error};
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

    // Get LoxBerry home directory from environment or use default
    let lbhomedir = std::env::var("LBHOMEDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/opt/loxberry"));

    let config_dir = lbhomedir.join("config/system");

    info!("Using LoxBerry directory: {}", lbhomedir.display());
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

    // Initialize MQTT Gateway if enabled
    let mqtt_gateway = if config.mqtt.udp_port() > 0 {
        info!("Initializing MQTT Gateway");
        match MqttGateway::new(config.mqtt.clone(), lbhomedir.clone()) {
            Ok(gateway) => {
                let gateway = Arc::new(gateway);

                // Start gateway in background
                let gateway_clone = Arc::clone(&gateway);
                tokio::spawn(async move {
                    if let Err(e) = gateway_clone.start().await {
                        error!("MQTT Gateway error: {}", e);
                    }
                });

                Some(gateway)
            }
            Err(e) => {
                warn!("Failed to initialize MQTT Gateway: {}", e);
                None
            }
        }
    } else {
        info!("MQTT Gateway disabled (udpinport = 0)");
        None
    };

    // Create application state
    let state = AppState::new(lbhomedir, config_manager, config, mqtt_gateway);

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
