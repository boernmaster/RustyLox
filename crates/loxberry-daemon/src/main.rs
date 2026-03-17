//! LoxBerry Daemon - Main entry point
//!
//! This is the main orchestrator binary that starts all services.

use anyhow::Result;
use auth::{AuditLogger, AuthService, AuthStore};
use loxberry_config::{ConfigManager, GeneralConfig};
use mqtt_gateway::MqttGateway;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use web_api::{create_router, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "loxberry_daemon=info,web_api=info,miniserver_client=info".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let version = env!("BUILD_VERSION");
    info!("Starting LoxBerry Daemon v{}", version);

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
            info!(
                "Loaded configuration from {}",
                config_manager.general_json_path().display()
            );
            cfg
        }
        Err(e) => {
            warn!("Failed to load configuration: {}", e);
            warn!("Using default configuration");
            GeneralConfig::default()
        }
    };

    // Wrap config in Arc<RwLock> for sharing between AppState and MqttGateway
    let config = Arc::new(RwLock::new(config));

    // Initialize MQTT Gateway if enabled
    let mqtt_gateway = {
        let config_read = config.read().await;
        if config_read.mqtt.udp_port() > 0 {
            drop(config_read);
            info!("Initializing MQTT Gateway");
            match MqttGateway::new(Arc::clone(&config), lbhomedir.clone()) {
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
        }
    };

    // Initialize authentication service
    let data_dir = lbhomedir.join("data/system");
    let log_dir = lbhomedir.join("log/system");
    let auth_store = AuthStore::new(&data_dir);
    let audit_logger = AuditLogger::new(&log_dir);
    let auth_service = AuthService::new(auth_store, audit_logger);
    match auth_service.init().await {
        Ok(()) => info!("Auth service initialized"),
        Err(e) => warn!("Auth service init warning: {}", e),
    }

    // Create application state
    let state = AppState::new_with_shared_config(
        lbhomedir,
        version.to_string(),
        config_manager,
        config,
        mqtt_gateway,
    )
    .with_auth(auth_service);

    // Create API router
    let api_router = create_router(state.clone());

    // Create UI router
    let ui_router = web_ui::create_ui_router(state.clone());

    // Merge routers - UI router serves the root, API router handles /api/*
    let app = ui_router.merge(api_router);

    // Get bind address from environment or use default
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    info!("Starting web server on http://{}", bind_addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    info!("LoxBerry Daemon is running!");
    info!("API available at: http://{}", bind_addr);
    info!("Health check: http://{}/health", bind_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
