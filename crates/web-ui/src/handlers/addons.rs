//! Addons page handlers (Catalog + Installed tabs)

use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::Form;
use chrono::Utc;

use addon_registry::proxy;

use crate::templates::{
    AddonSettingsFieldDisplay, AddonSettingsTemplate, AddonsTemplate, CatalogEntryDisplay,
    InstalledAddonDisplay,
};
use askama::Template;
use web_api::AppState;

/// List installed addons (self-registered) and the GHCR catalog
pub async fn list(State(state): State<AppState>) -> Html<String> {
    let installed = match &state.addon_registry {
        Some(registry) => registry
            .list(Utc::now())
            .await
            .into_iter()
            .map(|view| InstalledAddonDisplay {
                name: view.name,
                addon_version: view.version,
                online: view.online,
                dashboard_url: view.config_api_base_url,
            })
            .collect(),
        None => Vec::new(),
    };

    let (catalog_configured, catalog_entries) = match &state.catalog_client {
        Some(client) => {
            let entries = client
                .list_addons()
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|entry| CatalogEntryDisplay {
                    deploy_snippet: format!(
                        "docker pull ghcr.io/boernmaster/{}:latest",
                        entry.name
                    ),
                    name: entry.name,
                    title: entry.title,
                    description: entry.description,
                    source: entry.source,
                })
                .collect();
            (true, entries)
        }
        None => (false, Vec::new()),
    };

    let lang = state.config.read().await.base.lang.clone();
    let template = AddonsTemplate {
        installed,
        catalog_configured,
        catalog_entries,
        version: state.version.clone(),
        lang,
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// GET /addons/:name/settings
pub async fn settings(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let lang = state.config.read().await.base.lang.clone();

    let Some(registry) = &state.addon_registry else {
        return Html("<h1>Addons</h1><p>Addon registry not configured.</p>".to_string())
            .into_response();
    };
    let Some(instance) = registry.find(&name).await else {
        return (
            StatusCode::NOT_FOUND,
            Html(format!(
                "<h1>Addon not found</h1><p>No addon named '{}' is registered.</p>",
                name
            )),
        )
            .into_response();
    };

    let schema_result = proxy::fetch_schema(&instance.config_api_base_url).await;
    let config_result = proxy::fetch_config(&instance.config_api_base_url).await;

    let (offline, fields) = match (schema_result, config_result) {
        (Ok(schema), Ok(config)) => {
            let fields = schema
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|field| {
                    let key = field["key"].as_str().unwrap_or_default().to_string();
                    let secret = field["secret"].as_bool().unwrap_or(false);
                    let entry = config.get(&key).cloned().unwrap_or_default();
                    let value = if secret {
                        String::new()
                    } else {
                        entry["value"].as_str().unwrap_or_default().to_string()
                    };
                    AddonSettingsFieldDisplay {
                        label: field["label"].as_str().unwrap_or_default().to_string(),
                        help: field["help"].as_str().unwrap_or_default().to_string(),
                        input_type: if secret {
                            "password".to_string()
                        } else {
                            "text".to_string()
                        },
                        value,
                        secret_set: entry["secret_set"].as_bool().unwrap_or(false),
                        secret,
                        key,
                    }
                })
                .collect();
            (false, fields)
        }
        _ => (true, Vec::new()),
    };

    let template = AddonSettingsTemplate {
        addon_name: name,
        offline,
        fields,
        version: state.version.clone(),
        lang,
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
    .into_response()
}

/// POST /addons/:name/settings
///
/// Every field on the settings page has its own `<input>` pre-filled with its
/// current value (Step 1's `settings` handler), so a plain form submission
/// always contains every schema key - satisfying kia-connect-bridge's
/// full-object-replace `save_config` contract without any server-side merge
/// logic here. This handler is a pure forward, same as Task 7's proxy itself.
pub async fn settings_submit(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Form(fields): Form<HashMap<String, String>>,
) -> Html<String> {
    let Some(registry) = &state.addon_registry else {
        return Html(
            "<div class=\"alert alert-danger\">Addon registry not configured.</div>".to_string(),
        );
    };
    let Some(instance) = registry.find(&name).await else {
        return Html(format!(
            "<div class=\"alert alert-danger\">No addon named '{}' is registered.</div>",
            name
        ));
    };

    let payload = serde_json::Value::Object(
        fields
            .into_iter()
            .map(|(k, v)| (k, serde_json::Value::String(v)))
            .collect(),
    );

    match proxy::save_config(&instance.config_api_base_url, &payload).await {
        Ok(()) => Html("<div class=\"alert alert-success\">Settings saved.</div>".to_string()),
        Err(e) => Html(format!(
            "<div class=\"alert alert-danger\">Save failed: addon offline or unreachable ({}).</div>",
            e
        )),
    }
}
