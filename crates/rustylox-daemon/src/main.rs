//! LoxBerry Daemon - Main entry point
//!
//! This is the main orchestrator binary that starts all services.

use anyhow::Result;
use auth::{AuditLogger, AuthService, AuthStore};
use backup_manager::{scheduler::BackupSchedule as BmBackupSchedule, BackupScheduler};
use mqtt_gateway::MqttGateway;
use rustylox_config::{ConfigManager, GeneralConfig};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use web_api::{
    create_emu_router, create_router, weather::WeatherService, AppState, MiniserverEvent,
};

/// Default UDP port for receiving Miniserver Virtual UDP Output data.
/// Users point their Miniserver Virtual Output to `/dev/udp/<RustyLox-IP>/8090`.
const MINISERVER_UDP_RECV_PORT: u16 = 8090;

#[tokio::main]
async fn main() -> Result<()> {
    // Determine home dir early so we can set up file logging before any other init.
    let lbhomedir = std::env::var("LBHOMEDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/opt/loxberry"));

    let log_dir = lbhomedir.join("log/system");
    // Create log directory on first boot before writing to it.
    let _ = std::fs::create_dir_all(&log_dir);

    // Non-rolling file appender → log/system/rustylox.log.
    // Rotation is handled by the existing log-rotation task.
    let file_appender = tracing_appender::rolling::never(&log_dir, "rustylox.log");
    let (non_blocking_file, _file_guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "rustylox_daemon=info,web_api=info,miniserver_client=info".into());

    tracing_subscriber::registry()
        .with(env_filter)
        // Console layer: coloured output visible in `docker logs`
        .with(tracing_subscriber::fmt::layer())
        // File layer: plain text written to rustylox.log for the web log viewer
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking_file)
                .with_ansi(false),
        )
        .init();

    let version = env!("BUILD_VERSION");
    info!("Starting LoxBerry Daemon v{}", version);

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
        let mut w = cfg.weather.clone();
        // Resolve the miniserver IP from the Miniserver config map so UDP push works.
        if let Some(ms) = cfg.miniserver.get(&w.miniserver_key) {
            if !ms.ipaddress.is_empty() {
                w.miniserver_ip = ms.ipaddress.clone();
                info!(
                    "Weather UDP push target: {}:{}",
                    w.miniserver_ip, w.miniserver_udp_port
                );
            }
        }
        w
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

        // Bridge: inbound UDP (port 11884) → miniserver monitor
        // UdpReceived messages are currently only visible in the MQTT monitor;
        // this task also emits a MiniserverEvent so they appear in the miniserver monitor.
        {
            let monitor_tx = state.miniserver_monitor.clone();
            let mut gw_rx = gw.message_sender().subscribe();
            tokio::spawn(async move {
                loop {
                    match gw_rx.recv().await {
                        Ok(mqtt_gateway::GatewayMessage::UdpReceived { topic, value }) => {
                            let _ = monitor_tx.send(MiniserverEvent {
                                miniserver_id: 0,
                                miniserver_name: "Miniserver".to_string(),
                                direction: "received".to_string(),
                                protocol: "udp".to_string(),
                                url: Some("udp://:11884".to_string()),
                                params: Some(format!("{}={}", topic, value)),
                                response: None,
                                code: None,
                                error: None,
                                timestamp: chrono::Utc::now()
                                    .format("%Y-%m-%d %H:%M:%S")
                                    .to_string(),
                            });
                        }
                        Ok(_) => {}
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            warn!("Miniserver monitor bridge lagged by {} messages", n);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            });
        }
    }

    // Start Miniserver UDP receiver (for Virtual UDP Output from the Miniserver)
    {
        let recv_port = std::env::var("MS_UDP_RECV_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(MINISERVER_UDP_RECV_PORT);

        if recv_port > 0 {
            let bind_addr: std::net::SocketAddr = format!("0.0.0.0:{}", recv_port).parse().unwrap();
            let receiver = miniserver_client::MiniserverUdpReceiver::new(bind_addr, 256);
            let mut rx = receiver.subscribe();

            // Clone handles we need inside the spawned task
            let monitor_tx = state.miniserver_monitor.clone();
            let gateway_tx = state.mqtt_gateway.as_ref().map(|gw| gw.message_sender());

            match receiver.spawn().await {
                Ok(()) => {
                    info!(
                        "Miniserver UDP receiver listening on port {} (for Virtual UDP Output)",
                        recv_port
                    );

                    // Bridge received UDP messages → monitor + MQTT gateway
                    tokio::spawn(async move {
                        loop {
                            match rx.recv().await {
                                Ok(msg) => {
                                    // Parse payload into key=value pairs
                                    let (prefix, pairs) =
                                        miniserver_client::parse_udp_payload(&msg.payload);

                                    // Emit monitor event
                                    let _ = monitor_tx.send(MiniserverEvent {
                                        miniserver_id: 0,
                                        miniserver_name: prefix
                                            .clone()
                                            .unwrap_or_else(|| "Miniserver UDP".to_string()),
                                        direction: "received".to_string(),
                                        protocol: "udp".to_string(),
                                        url: Some(msg.from.to_string()),
                                        params: Some(msg.payload.clone()),
                                        response: None,
                                        code: None,
                                        error: None,
                                        timestamp: chrono::Utc::now()
                                            .format("%Y-%m-%d %H:%M:%S")
                                            .to_string(),
                                    });

                                    // Forward each pair to the MQTT gateway
                                    if let Some(ref gw_tx) = gateway_tx {
                                        for (key, value) in &pairs {
                                            let topic = if let Some(ref pfx) = prefix {
                                                format!("{}/{}", pfx, key)
                                            } else {
                                                key.clone()
                                            };
                                            let gw_msg =
                                                mqtt_gateway::GatewayMessage::UdpReceived {
                                                    topic,
                                                    value: value.clone(),
                                                };
                                            let _ = gw_tx.send(gw_msg);
                                        }
                                    }
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                    warn!("Miniserver UDP receiver lagged by {} messages", n);
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                    break;
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    warn!(
                        "Failed to start Miniserver UDP receiver on port {}: {}",
                        recv_port, e
                    );
                }
            }
        }
    }

    // Create API router
    let api_router = create_router(state.clone());

    // Spawn system backup scheduler (reads schedule from general.json config)
    {
        let sched_cfg = {
            let cfg = state.config.read().await;
            BmBackupSchedule {
                enabled: cfg.backup.schedule.active == "true",
                interval_hours: cfg.backup.schedule.interval_hours,
                include_plugins: cfg.backup.schedule.include_plugins,
                max_backups: cfg.backup.schedule.keep_backups,
            }
        };
        let backup_scheduler =
            BackupScheduler::new(state.lbhomedir.clone(), version.to_string(), sched_cfg);
        tokio::spawn(async move {
            if let Err(e) = backup_scheduler.run().await {
                error!("Backup scheduler error: {}", e);
            }
        });
    }

    // Spawn the cron-based task scheduler background loop
    {
        let task_scheduler = Arc::new(task_scheduler::TaskScheduler::new(
            &state.lbhomedir,
            version,
        ));
        task_scheduler.start_background_scheduler();
    }

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
            let app_emu = create_emu_router(state.clone());
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
