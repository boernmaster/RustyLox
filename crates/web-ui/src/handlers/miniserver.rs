//! Miniserver management handlers

use crate::templates::{
    MiniserverDisplay, MiniserverEditTemplate, MiniserverForm, MiniserverListTemplate,
    MiniserverMessage, MiniserverMonitorTemplate,
};
use askama::Template;
use axum::response::sse::{Event, KeepAlive};
use axum::{
    extract::{Path, State},
    response::{Html, Sse},
    Form,
};
use futures::stream::Stream;
use serde::Deserialize;
use std::convert::Infallible;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use web_api::AppState;

/// List all Miniservers
pub async fn list(State(state): State<AppState>) -> Html<String> {
    let config = state.config.read().await;

    let miniservers: Vec<MiniserverDisplay> = config
        .miniserver
        .iter()
        .map(|(id, ms)| MiniserverDisplay {
            id: id.clone(),
            name: ms.name.clone(),
            ipaddress: ms.ipaddress.clone(),
            port: ms.port.clone(),
            connected: false, // TODO: Check actual connection status
        })
        .collect();

    let template = MiniserverListTemplate { miniservers };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// Show add Miniserver form
pub async fn add_form(State(_state): State<AppState>) -> Html<String> {
    let template = MiniserverEditTemplate {
        miniserver: None,
        is_new: true,
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

#[derive(Debug, Deserialize)]
pub struct MiniserverFormData {
    pub name: String,
    pub ipaddress: String,
    pub port: String,
    pub admin: String,
    pub pass: String,
    pub useclouddns: Option<String>,
}

/// Submit add Miniserver
pub async fn add_submit(
    State(state): State<AppState>,
    Form(form): Form<MiniserverFormData>,
) -> Html<String> {
    // Get mutable config
    let mut config = state.config.write().await;

    // Find next available ID
    let next_id = (1..=99)
        .find(|i| !config.miniserver.contains_key(&i.to_string()))
        .unwrap_or(1)
        .to_string();

    // Build credentials
    let credentials_raw = format!("{}:{}", form.admin, form.pass);
    let transport = "http".to_string();
    let port = form.port.clone();
    let useclouddns = if form.useclouddns.is_some() { "1" } else { "0" }.to_string();

    // Create new Miniserver config
    let ms_config = loxberry_config::MiniserverConfig {
        name: form.name.clone(),
        ipaddress: form.ipaddress.clone(),
        port: port.clone(),
        admin: form.admin.clone(),
        admin_raw: form.admin.clone(),
        pass: form.pass.clone(),
        pass_raw: form.pass.clone(),
        credentials: credentials_raw.clone(),
        credentials_raw: credentials_raw.clone(),
        transport: transport.clone(),
        useclouddns: useclouddns.clone(),
        fulluri: format!(
            "{}://{}@{}:{}",
            transport, credentials_raw, form.ipaddress, port
        ),
        fulluri_raw: format!(
            "{}://{}@{}:{}",
            transport, credentials_raw, form.ipaddress, port
        ),
        ..Default::default()
    };

    // Add to config
    config.miniserver.insert(next_id.clone(), ms_config);

    // Save configuration
    match state.config_manager.save_general(&config).await {
        Ok(_) => {
            drop(config); // Release lock before reloading
            let _ = state.reload_config().await;
            Html(format!(
                "<div class='alert alert-success'>Miniserver '{}' added successfully. <a href='/miniserver'>Back to list</a></div>",
                form.name
            ))
        }
        Err(e) => Html(format!(
            "<div class='alert alert-danger'>Error saving configuration: {}</div>",
            e
        )),
    }
}

/// Show edit Miniserver form
pub async fn edit_form(State(state): State<AppState>, Path(id): Path<String>) -> Html<String> {
    let config = state.config.read().await;

    let miniserver = config.miniserver.get(&id).map(|ms| MiniserverForm {
        id: Some(id.clone()),
        name: ms.name.clone(),
        ipaddress: ms.ipaddress.clone(),
        port: ms.port.clone(),
        admin: ms.admin.clone(),
        pass: ms.pass.clone(),
        useclouddns: ms.useclouddns == "1",
    });

    let template = MiniserverEditTemplate {
        miniserver,
        is_new: false,
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// Submit edit Miniserver
pub async fn edit_submit(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Form(form): Form<MiniserverFormData>,
) -> Html<String> {
    // Get mutable config
    let mut config = state.config.write().await;

    // Check if Miniserver exists
    if !config.miniserver.contains_key(&id) {
        return Html(format!(
            "<div class='alert alert-danger'>Miniserver not found. <a href='/miniserver'>Back to list</a></div>"
        ));
    }

    // Build credentials
    let credentials_raw = format!("{}:{}", form.admin, form.pass);
    let transport = "http".to_string();
    let port = form.port.clone();
    let useclouddns = if form.useclouddns.is_some() { "1" } else { "0" }.to_string();

    // Update Miniserver config
    let ms_config = loxberry_config::MiniserverConfig {
        name: form.name.clone(),
        ipaddress: form.ipaddress.clone(),
        port: port.clone(),
        admin: form.admin.clone(),
        admin_raw: form.admin.clone(),
        pass: form.pass.clone(),
        pass_raw: form.pass.clone(),
        credentials: credentials_raw.clone(),
        credentials_raw: credentials_raw.clone(),
        transport: transport.clone(),
        useclouddns: useclouddns.clone(),
        fulluri: format!(
            "{}://{}@{}:{}",
            transport, credentials_raw, form.ipaddress, port
        ),
        fulluri_raw: format!(
            "{}://{}@{}:{}",
            transport, credentials_raw, form.ipaddress, port
        ),
        ..Default::default()
    };

    // Update in config
    config.miniserver.insert(id.clone(), ms_config);

    // Save configuration
    match state.config_manager.save_general(&config).await {
        Ok(_) => {
            drop(config); // Release lock
            let _ = state.reload_config().await;
            // Clear cached client so it gets recreated with new credentials
            state
                .miniserver_clients
                .remove(&id.parse::<u8>().unwrap_or(1));
            Html(format!(
                "<div class='alert alert-success'>Miniserver '{}' updated successfully. <a href='/miniserver'>Back to list</a></div>",
                form.name
            ))
        }
        Err(e) => Html(format!(
            "<div class='alert alert-danger'>Error saving configuration: {}</div>",
            e
        )),
    }
}

/// Delete Miniserver
pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> Html<String> {
    // Get mutable config
    let mut config = state.config.write().await;

    // Remove from config
    if config.miniserver.remove(&id).is_some() {
        // Save configuration
        match state.config_manager.save_general(&config).await {
            Ok(_) => {
                drop(config); // Release lock
                let _ = state.reload_config().await;
                // Remove cached client
                state
                    .miniserver_clients
                    .remove(&id.parse::<u8>().unwrap_or(1));
                Html("<div class='alert alert-success'>Miniserver deleted. <a href='/miniserver'>Back to list</a></div>".to_string())
            }
            Err(e) => Html(format!(
                "<div class='alert alert-danger'>Error saving configuration: {}</div>",
                e
            )),
        }
    } else {
        Html("<div class='alert alert-danger'>Miniserver not found.</div>".to_string())
    }
}

/// Test Miniserver connection
pub async fn test_connection(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Html<String> {
    // Parse ID
    let ms_id = match id.parse::<u8>() {
        Ok(id) => id,
        Err(_) => {
            return Html("<div class='alert alert-danger'>Invalid Miniserver ID</div>".to_string())
        }
    };

    // Get or create client
    match state.get_miniserver_client(ms_id).await {
        Ok(client) => {
            // Try to send a simple command (get status)
            match client.get(vec!["status".to_string()]).await {
                Ok(_) => Html(
                    "<div class='alert alert-success'>✓ Connection test successful!</div>"
                        .to_string(),
                ),
                Err(e) => Html(format!(
                    "<div class='alert alert-danger'>✗ Connection test failed: {}</div>",
                    e
                )),
            }
        }
        Err(e) => Html(format!(
            "<div class='alert alert-danger'>✗ Failed to create client: {}</div>",
            e
        )),
    }
}

/// Miniserver Monitor page (displays the UI)
pub async fn monitor(State(_state): State<AppState>) -> Html<String> {
    let template = MiniserverMonitorTemplate {
        title: "Miniserver Monitor - Real-time Communication Viewer".to_string(),
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// Miniserver Monitor real-time stream (Server-Sent Events)
/// This streams Miniserver communication in real-time to the browser
pub async fn monitor_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Create a channel for forwarding messages to UI
    let (tx, rx) = broadcast::channel::<MiniserverMessage>(100);

    // For now, we'll send a placeholder message
    // TODO: In Phase 5, integrate with actual miniserver-client to broadcast messages
    tokio::spawn(async move {
        // Send initial status message
        let _ = tx.send(MiniserverMessage {
            direction: "received".to_string(),
            protocol: "http".to_string(),
            miniserver_name: "Monitoring Active".to_string(),
            url: None,
            params: None,
            response: Some("Miniserver monitor is running. Actual messages will appear here when Miniserver communication occurs.".to_string()),
            code: Some("200".to_string()),
            error: None,
            timestamp: chrono::Utc::now()
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
        });
    });

    // Convert broadcast channel to SSE stream
    let stream = BroadcastStream::new(rx).map(|result| match result {
        Ok(msg) => {
            // Serialize message to JSON for the client
            let json = serde_json::to_string(&msg).unwrap_or_default();
            Ok(Event::default().data(json))
        }
        Err(_) => {
            // Channel closed
            Ok(Event::default().data("Miniserver monitor not available"))
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
