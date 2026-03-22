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
use web_api::{create_router, weather::WeatherService, AppState, MiniserverEvent};

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

    // Initialize weather service
    let weather_cfg = {
        let cfg = config.read().await;
        cfg.weather.clone()
    };
    let weather_service = Arc::new(WeatherService::new(weather_cfg));

    // Spawn background refresh loop (shares the same Arc)
    Arc::clone(&weather_service).spawn_background_task();

    // Create application state
    let state = AppState::new_with_shared_config(
        lbhomedir,
        version.to_string(),
        config_manager,
        config,
        mqtt_gateway,
    )
    .with_auth(auth_service)
    .with_weather(Arc::clone(&weather_service));

    // Wire MQTT gateway relay to the miniserver monitor so outbound sends appear in the UI
    if let Some(gw) = &state.mqtt_gateway {
        let tx = state.miniserver_monitor.clone();
        let callback: miniserver_client::MonitorCallback =
            std::sync::Arc::new(move |event: miniserver_client::MonitorEvent| {
                let _ = tx.send(MiniserverEvent {
                    miniserver_id: 0,
                    miniserver_name: "MQTT Gateway".to_string(),
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
        gw.set_miniserver_monitor(callback).await;
    }

    // Create API router
    let api_router = create_router(state.clone());

    // Spawn Miniserver backup scheduler
    web_ui::handlers::miniserver_backup::spawn_ms_backup_scheduler(state.clone());

    // Create UI router
    let ui_router = web_ui::create_ui_router(state.clone());

    // Merge routers - UI router serves the root, API router handles /api/*
    let app = ui_router.merge(api_router);

    // Get bind address from environment or use default (0.0.0.0 = all interfaces)
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    // Extract the port so we can build a human-readable LAN URL
    let port = bind_addr.rsplit(':').next().unwrap_or("8080").to_string();

    // Detect LAN IP by connecting a UDP socket (no packet is actually sent)
    let lan_ip = std::net::UdpSocket::bind("0.0.0.0:0")
        .ok()
        .and_then(|s| {
            s.connect("8.8.8.8:80").ok()?;
            s.local_addr().ok()
        })
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string());

    // Start main server
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    // Also start port 6066 for Loxone Cloud Emulator (weather.loxone.com)
    let emu_addr = "0.0.0.0:6066";
    match tokio::net::TcpListener::bind(emu_addr).await {
        Ok(emu_listener) => {
            info!("Loxone Cloud Emulator listening on port 6066");
            let app_emu =
                create_router(state.clone()).merge(web_ui::create_ui_router(state.clone()));
            tokio::spawn(async move {
                if let Err(e) = axum::serve(emu_listener, app_emu).await {
                    error!("Port 6066 server error: {}", e);
                }
            });
        }
        Err(e) => {
            warn!(
                "Could not bind port 6066 (Loxone EMU): {} – feature disabled",
                e
            );
        }
    }

    info!("LoxBerry Daemon is running!");
    info!("Local:   http://localhost:{}", port);
    info!("Network: http://{}:{}", lan_ip, port);
    info!("Health:  http://{}:{}/health", lan_ip, port);

    axum::serve(listener, app).await?;

    Ok(())
}
