//! MQTT handlers including real-time monitor

use crate::templates::{MqttConfigForm, MqttConfigTemplate, MqttMessage, MqttMonitorTemplate};
use askama::Template;
use axum::{
    extract::State,
    response::{Html, Sse},
    Form,
};
use axum::response::sse::{Event, KeepAlive};
use futures::stream::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use web_api::AppState;

/// MQTT Monitor page (displays the UI)
pub async fn monitor(State(_state): State<AppState>) -> Html<String> {
    let template = MqttMonitorTemplate {
        title: "MQTT Monitor - Real-time Message Viewer".to_string(),
    };

    Html(template.render().unwrap_or_else(|_| "Error rendering template".to_string()))
}

/// MQTT Monitor real-time stream (Server-Sent Events)
/// This streams MQTT messages in real-time to the browser
pub async fn monitor_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Create a channel for MQTT messages
    let (tx, rx) = broadcast::channel::<MqttMessage>(100);

    // Check if MQTT gateway is available
    if let Some(mqtt_gateway) = &state.mqtt_gateway {
        // Clone for spawned task
        let _gateway = mqtt_gateway.clone();

        // Spawn a task to receive MQTT messages and forward them
        // This is a simplified version - in production, integrate with gateway's broadcast
        tokio::spawn(async move {
            // Simulate receiving messages (replace with actual gateway subscription)
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;

                let msg = MqttMessage {
                    topic: "system/heartbeat".to_string(),
                    payload: format!("{{\"timestamp\":\"{}\"}}", chrono::Utc::now().to_rfc3339()),
                    qos: 0,
                    retain: false,
                    timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                };

                if tx.send(msg).is_err() {
                    break;
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
        brokeruser: String::new(), // TODO: Add to MqttConfig
        brokerpass: String::new(), // TODO: Add to MqttConfig
        udpinport: config.mqtt.udpinport.clone(),
    };

    let template = MqttConfigTemplate {
        config: mqtt_config,
    };

    Html(template.render().unwrap_or_else(|_| "Error rendering template".to_string()))
}

#[derive(Debug, Deserialize)]
pub struct MqttConfigFormData {
    pub brokerhost: String,
    pub brokerport: String,
    pub brokeruser: String,
    pub brokerpass: String,
    pub udpinport: String,
}

/// Submit MQTT configuration
pub async fn config_submit(
    State(state): State<AppState>,
    Form(form): Form<MqttConfigFormData>,
) -> Html<String> {
    // TODO: Update configuration and save to file
    // TODO: Restart MQTT gateway with new config

    Html("<div class='success'>MQTT configuration updated successfully</div>".to_string())
}

/// MQTT Subscriptions page
pub async fn subscriptions(State(state): State<AppState>) -> Html<String> {
    // TODO: Load subscriptions from gateway
    Html("<div>Subscriptions management page</div>".to_string())
}

/// Add MQTT subscription
pub async fn add_subscription(
    State(state): State<AppState>,
    Form(form): Form<SubscriptionForm>,
) -> Html<String> {
    // TODO: Add subscription to gateway and save
    Html("<div class='success'>Subscription added</div>".to_string())
}

/// Delete MQTT subscription
pub async fn delete_subscription(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Html<String> {
    // TODO: Delete subscription from gateway
    Html("<div class='success'>Subscription deleted</div>".to_string())
}

#[derive(Debug, Deserialize)]
pub struct SubscriptionForm {
    pub topic: String,
    pub name: String,
}
