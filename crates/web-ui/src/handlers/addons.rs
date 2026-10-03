//! Addons page handlers (Catalog + Installed tabs)

use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::Form;
use chrono::Utc;

use addon_registry::proxy;

use crate::templates::{
    AddonSettingsFieldDisplay, AddonSettingsTemplate, AddonUiTemplate, AddonsTemplate,
    CatalogEntryDisplay, InstalledAddonDisplay,
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

/// GET /addons/:name/ui
pub async fn ui(State(state): State<AppState>, Path(name): Path<String>) -> impl IntoResponse {
    let lang = state.config.read().await.base.lang.clone();

    let Some(registry) = &state.addon_registry else {
        return Html("<h1>Addons</h1><p>Addon registry not configured.</p>".to_string())
            .into_response();
    };
    let Some(instance) = registry.find(&name).await else {
        return (StatusCode::NOT_FOUND, Html(addon_not_found_page(&name))).into_response();
    };

    let template = AddonUiTemplate {
        addon_name: name,
        dashboard_url: instance.config_api_base_url,
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
        return (StatusCode::NOT_FOUND, Html(addon_not_found_page(&name))).into_response();
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
        return Html(addon_not_found_alert(&name));
    };

    let payload = serde_json::Value::Object(
        fields
            .into_iter()
            .map(|(k, v)| (k, serde_json::Value::String(v)))
            .collect(),
    );

    match proxy::save_config(&instance.config_api_base_url, &payload).await {
        Ok(()) => Html("<div class=\"alert alert-success\">Settings saved.</div>".to_string()),
        Err(e) => Html(save_failure_html(&e)),
    }
}

/// The alert shown on the settings page when saving did not work.
fn save_failure_html(error: &proxy::ProxyError) -> String {
    match error {
        proxy::ProxyError::Rejected(reason) => format!(
            "<div class=\"alert alert-danger\">Not saved: {}</div>",
            html_escape(reason)
        ),
        other => format!(
            "<div class=\"alert alert-danger\">Save failed: addon offline or unreachable ({}).</div>",
            html_escape(&other.to_string())
        ),
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Full-page answer for an addon name taken from the URL that is not registered.
fn addon_not_found_page(name: &str) -> String {
    format!(
        "<h1>Addon not found</h1><p>No addon named '{}' is registered.</p>",
        html_escape(name)
    )
}

/// Same as [`addon_not_found_page`], as an alert for the HTMX save target.
fn addon_not_found_alert(name: &str) -> String {
    format!(
        "<div class=\"alert alert-danger\">No addon named '{}' is registered.</div>",
        html_escape(name)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rejected_save_shows_the_addons_reason_instead_of_claiming_it_is_offline() {
        let html = save_failure_html(&proxy::ProxyError::Rejected(
            "Changing MQTT_HOST requires re-entering MQTT_PASSWORD".to_string(),
        ));

        assert!(html.contains("Changing MQTT_HOST requires re-entering MQTT_PASSWORD"));
        assert!(!html.contains("offline"), "{html}");
    }

    /// The reason comes from the addon, i.e. from outside RustyLox.
    #[test]
    fn the_addons_reason_is_html_escaped() {
        let html = save_failure_html(&proxy::ProxyError::Rejected(
            "<script>alert(1)</script>".to_string(),
        ));

        assert!(!html.contains("<script>"), "{html}");
    }

    /// The name is whatever was typed into the URL.
    #[test]
    fn an_unknown_addon_name_is_html_escaped_on_the_not_found_page() {
        let html = addon_not_found_page("<script>alert(1)</script>");

        assert!(!html.contains("<script>"), "{html}");
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn an_unknown_addon_name_is_html_escaped_in_the_save_alert() {
        let html = addon_not_found_alert("<script>alert(1)</script>");

        assert!(!html.contains("<script>"), "{html}");
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn an_unreachable_addon_is_still_reported_as_offline() {
        let html = save_failure_html(&proxy::ProxyError::Unreachable("timeout".to_string()));

        assert!(html.contains("offline or unreachable"));
    }
}
