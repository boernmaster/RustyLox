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

    let template = PluginListTemplate {
        plugins,
        version: state.version.clone(),
    };

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
pub async fn install_submit(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Html<String> {
    // Extract uploaded file from multipart
    let mut zip_path: Option<std::path::PathBuf> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            // Get filename from content disposition
            let filename = field.file_name().unwrap_or("upload.zip").to_string();

            // Validate ZIP extension
            if !filename.to_lowercase().ends_with(".zip") {
                return Html(String::from(
                    "<div class='error'>Only .zip files are allowed</div>",
                ));
            }

            // Read file data
            let data = match field.bytes().await {
                Ok(bytes) => bytes,
                Err(e) => {
                    return Html(format!(
                        "<div class='error'>Failed to read uploaded file: {}</div>",
                        e
                    ));
                }
            };

            // Save to temporary location
            let temp_dir = state.lbhomedir.join("tmp");
            if let Err(e) = tokio::fs::create_dir_all(&temp_dir).await {
                return Html(format!(
                    "<div class='error'>Failed to create temp directory: {}</div>",
                    e
                ));
            }

            let temp_path = temp_dir.join(&filename);
            if let Err(e) = tokio::fs::write(&temp_path, &data).await {
                return Html(format!(
                    "<div class='error'>Failed to save uploaded file: {}</div>",
                    e
                ));
            }

            zip_path = Some(temp_path);
            break;
        }
    }

    // Check if we got a file
    let zip_path = match zip_path {
        Some(path) => path,
        None => {
            return Html(String::from("<div class='error'>No file uploaded</div>"));
        }
    };

    // Install the plugin
    let plugin_manager = plugin_manager::PluginInstaller::new(&state.lbhomedir);

    let install_request = plugin_manager::InstallRequest {
        zip_path: zip_path.clone(),
        action: plugin_manager::InstallAction::Install,
        force: false,
    };

    match plugin_manager.install(install_request).await {
        Ok(plugin) => {
            // Clean up temp file
            let _ = tokio::fs::remove_file(&zip_path).await;

            Html(format!(
                "<div class='success'>Plugin <strong>{}</strong> v{} installed successfully. \
                 <a href='/plugins'>Back to list</a></div>",
                plugin.name, plugin.version
            ))
        }
        Err(e) => {
            // Clean up temp file
            let _ = tokio::fs::remove_file(&zip_path).await;

            Html(format!(
                "<div class='error'>Failed to install plugin: {}</div>",
                e
            ))
        }
    }
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
