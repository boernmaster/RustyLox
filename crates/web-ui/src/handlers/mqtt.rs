//! MQTT handlers including real-time monitor

use crate::templates::{MqttConfigForm, MqttConfigTemplate, MqttMessage, MqttMonitorTemplate};
use askama::Template;
use axum::response::sse::{Event, KeepAlive};
use axum::{
    extract::State,
    response::{Html, Sse},
    Form,
};
use futures::stream::{Stream, StreamExt};
use serde::Deserialize;
use std::convert::Infallible;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use web_api::AppState;

/// MQTT Monitor page — "Incoming Overview" showing relay state to Miniserver
pub async fn monitor(State(state): State<AppState>) -> Html<String> {
    let template = MqttMonitorTemplate {
        title: "Incoming Overview".to_string(),
        version: state.version.clone(),
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// MQTT Monitor real-time stream (Server-Sent Events)
/// This streams MQTT messages in real-time to the browser
pub async fn monitor_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Create a channel for forwarding messages to UI
    let (tx, rx) = broadcast::channel::<MqttMessage>(100);

    // Check if MQTT gateway is available
    if let Some(mqtt_gateway) = &state.mqtt_gateway {
        // Subscribe to gateway messages
        let mut gateway_rx = mqtt_gateway.subscribe_messages();

        // Spawn a task to receive gateway messages and forward to UI
        tokio::spawn(async move {
            loop {
                match gateway_rx.recv().await {
                    Ok(gateway_msg) => {
                        // Convert GatewayMessage to MqttMessage for UI
                        let ui_msg = match gateway_msg {
                            mqtt_gateway::GatewayMessage::MqttReceived { topic, payload } => {
                                Some(MqttMessage {
                                    topic,
                                    payload: String::from_utf8_lossy(&payload).to_string(),
                                    qos: 0,
                                    retain: false,
                                    timestamp: chrono::Utc::now()
                                        .format("%Y-%m-%d %H:%M:%S")
                                        .to_string(),
                                })
                            }
                            mqtt_gateway::GatewayMessage::UdpReceived { topic, value } => {
                                Some(MqttMessage {
                                    topic,
                                    payload: value,
                                    qos: 0,
                                    retain: false,
                                    timestamp: chrono::Utc::now()
                                        .format("%Y-%m-%d %H:%M:%S")
                                        .to_string(),
                                })
                            }
                            _ => None, // Ignore other message types
                        };

                        if let Some(msg) = ui_msg {
                            if tx.send(msg).is_err() {
                                break; // UI disconnected
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("MQTT monitor lagged by {} messages", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break; // Gateway stopped
                    }
                }
            }
        });
    } else {
        // No MQTT gateway - close the channel immediately
        drop(tx);
    }

    // Convert broadcast channel to SSE stream
    let stream = BroadcastStream::new(rx).map(|result| {
        match result {
            Ok(msg) => {
                // Serialize message to JSON for the client
                let json = serde_json::to_string(&msg).unwrap_or_default();
                Ok(Event::default().data(json))
            }
            Err(_) => {
                // Channel closed
                Ok(Event::default().data("MQTT Gateway not available"))
            }
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// MQTT Configuration page
pub async fn config(State(state): State<AppState>) -> Html<String> {
    let config = state.config.read().await;

    let mqtt_config = MqttConfigForm {
        brokerhost: config.mqtt.brokerhost.clone(),
        brokerport: config.mqtt.brokerport.clone(),
        brokeruser: config.mqtt.brokeruser.clone(),
        brokerpass: config.mqtt.brokerpass.clone(),
        udpinport: config.mqtt.udpinport.clone(),
        topicfilter: config.mqtt.topicfilter.clone(),
    };

    let template = MqttConfigTemplate {
        config: mqtt_config,
        version: state.version.clone(),
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

#[derive(Debug, Deserialize)]
pub struct MqttConfigFormData {
    pub brokerhost: String,
    pub brokerport: String,
    pub brokeruser: String,
    pub brokerpass: String,
    pub udpinport: String,
    #[serde(default)]
    pub topicfilter: String,
}

/// Submit MQTT configuration
pub async fn config_submit(
    State(state): State<AppState>,
    Form(form): Form<MqttConfigFormData>,
) -> Html<String> {
    // Build config for validation before writing
    let candidate = rustylox_config::MqttConfig {
        brokerhost: form.brokerhost.clone(),
        brokerport: form.brokerport.clone(),
        brokeruser: form.brokeruser.clone(),
        brokerpass: form.brokerpass.clone(),
        udpinport: form.udpinport.clone(),
        topicfilter: form.topicfilter.clone(),
        ..Default::default()
    };

    if let Err(e) = rustylox_config::validation::validate_mqtt_config(&candidate) {
        return Html(format!(
            "<div class='alert alert-danger'>Validation error: {}</div>",
            e
        ));
    }

    // Get mutable config and apply validated values
    let mut config = state.config.write().await;
    config.mqtt.brokerhost = form.brokerhost;
    config.mqtt.brokerport = form.brokerport;
    config.mqtt.brokeruser = form.brokeruser;
    config.mqtt.brokerpass = form.brokerpass;
    config.mqtt.udpinport = form.udpinport;
    config.mqtt.topicfilter = form.topicfilter;

    // Save configuration
    match state.config_manager.save_general(&config).await {
        Ok(_) => {
            drop(config); // Release lock
            let _ = state.reload_config().await;
            Html("<div class='alert alert-success'>MQTT configuration updated successfully. Restart required for changes to take effect.</div>".to_string())
        }
        Err(e) => Html(format!(
            "<div class='alert alert-danger'>Error saving configuration: {}</div>",
            e
        )),
    }
}
