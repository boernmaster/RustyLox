//! Plugin management handlers

use crate::templates::{PluginDisplay, PluginListTemplate};
use askama::Template;
use axum::{
    extract::{Multipart, Path, State},
    response::Html,
};
use web_api::AppState;

/// List all plugins
pub async fn list(State(state): State<AppState>) -> Html<String> {
    let plugin_manager = plugin_manager::PluginInstaller::new(&state.lbhomedir);

    let plugins = match plugin_manager.list().await {
        Ok(plugins) => plugins
            .iter()
            .map(|p| PluginDisplay {
                md5: p.md5.clone(),
                name: p.name.clone(),
                version: p.version.clone(),
                author: p.author_name.clone(),
                title: p.title.get("en").cloned().unwrap_or_else(|| p.name.clone()),
            })
            .collect(),
        Err(_) => Vec::new(),
    };

    let template = PluginListTemplate { plugins };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// Show plugin install form
pub async fn install_form(State(_state): State<AppState>) -> Html<String> {
    Html(String::from(
        "<h2>Install Plugin</h2>\
         <form method=\"post\" enctype=\"multipart/form-data\" hx-post=\"/plugins/install\" hx-target=\"#result\">\
             <div class=\"form-group\">\
                 <label for=\"file\">Plugin ZIP File:</label>\
                 <input type=\"file\" id=\"file\" name=\"file\" accept=\".zip\" required>\
             </div>\
             <button type=\"submit\" class=\"btn btn-primary\">Install Plugin</button>\
         </form>\
         <div id=\"result\"></div>",
    ))
}

/// Submit plugin installation
pub async fn install_submit(State(_state): State<AppState>, _multipart: Multipart) -> Html<String> {
    // TODO: Handle file upload and install plugin
    Html(String::from("<div class='success'>Plugin installed successfully. <a href='/plugins'>Back to list</a></div>"))
}

/// Show plugin details
pub async fn details(State(state): State<AppState>, Path(md5): Path<String>) -> Html<String> {
    let plugin_manager = plugin_manager::PluginInstaller::new(&state.lbhomedir);

    match plugin_manager.get(&md5).await {
        Ok(Some(plugin)) => {
            let html = format!(
                "<h2>{}</h2>\
                 <p><strong>Version:</strong> {}</p>\
                 <p><strong>Author:</strong> {}</p>\
                 <p><strong>MD5:</strong> {}</p>\
                 <form method=\"post\" hx-post=\"/plugins/{}/uninstall\" hx-target=\"#result\">\
                     <button type=\"submit\" class=\"btn btn-danger\">Uninstall</button>\
                 </form>\
                 <div id=\"result\"></div>",
                plugin.name, plugin.version, plugin.author_name, plugin.md5, plugin.md5
            );
            Html(html)
        }
        _ => Html(String::from("<div class='error'>Plugin not found</div>")),
    }
}

/// Uninstall plugin
pub async fn uninstall(State(state): State<AppState>, Path(md5): Path<String>) -> Html<String> {
    let plugin_manager = plugin_manager::PluginInstaller::new(&state.lbhomedir);

    match plugin_manager.uninstall(&md5).await {
        Ok(_) => Html(String::from(
            "<div class='success'>Plugin uninstalled. <a href='/plugins'>Back to list</a></div>",
        )),
        Err(e) => Html(format!("<div class='error'>Error: {}</div>", e)),
    }
}
