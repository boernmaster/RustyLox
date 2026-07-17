//! Addons page handlers (Catalog + Installed tabs)

use axum::extract::State;
use axum::response::Html;
use chrono::Utc;

use crate::templates::{AddonsTemplate, CatalogEntryDisplay, InstalledAddonDisplay};
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
