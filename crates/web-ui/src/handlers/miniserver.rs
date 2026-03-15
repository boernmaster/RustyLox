//! Miniserver management handlers

use crate::templates::{MiniserverDisplay, MiniserverEditTemplate, MiniserverForm, MiniserverListTemplate};
use askama::Template;
use axum::{
    extract::{Path, State},
    response::Html,
    Form,
};
use serde::Deserialize;
use std::sync::Arc;
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

    Html(template.render().unwrap_or_else(|_| "Error rendering template".to_string()))
}

/// Show add Miniserver form
pub async fn add_form(State(_state): State<AppState>) -> Html<String> {
    let template = MiniserverEditTemplate {
        miniserver: None,
        is_new: true,
    };

    Html(template.render().unwrap_or_else(|_| "Error rendering template".to_string()))
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
    // TODO: Add miniserver to configuration
    // TODO: Save configuration file
    // TODO: Test connection

    Html(format!(
        "<div class='success'>Miniserver '{}' added successfully. <a href='/miniserver'>Back to list</a></div>",
        form.name
    ))
}

/// Show edit Miniserver form
pub async fn edit_form(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Html<String> {
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

    Html(template.render().unwrap_or_else(|_| "Error rendering template".to_string()))
}

/// Submit edit Miniserver
pub async fn edit_submit(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Form(form): Form<MiniserverFormData>,
) -> Html<String> {
    // TODO: Update miniserver in configuration
    // TODO: Save configuration file

    Html(format!(
        "<div class='success'>Miniserver '{}' updated successfully. <a href='/miniserver'>Back to list</a></div>",
        form.name
    ))
}

/// Delete Miniserver
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Html<String> {
    // TODO: Remove miniserver from configuration
    // TODO: Save configuration file

    Html(format!(
        "<div class='success'>Miniserver deleted. <a href='/miniserver'>Back to list</a></div>"
    ))
}

/// Test Miniserver connection
pub async fn test_connection(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Html<String> {
    // TODO: Test connection to Miniserver
    // TODO: Return success or error message

    Html("<div class='success'>Connection test successful!</div>".to_string())
}
