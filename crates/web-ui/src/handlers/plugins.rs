//! Plugin management handlers

use crate::templates::{
    PluginDetailsTemplate, PluginDisplay, PluginInstallTemplate, PluginListTemplate,
};
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
        Ok(plugins) => {
            let mut displays = Vec::new();
            for p in plugins {
                // Check if plugin has daemon
                let daemon_dir = state
                    .lbhomedir
                    .join("bin/plugins")
                    .join(&p.folder)
                    .join("daemon");
                let has_daemon = daemon_dir.exists();

                // Check daemon status via pidfile at run/plugins/{folder}/{folder}.pid
                let daemon_running = if has_daemon {
                    is_daemon_running(&state.lbhomedir, &p.folder)
                } else {
                    false
                };

                // Check if plugin has web UI
                let webui_dir = state
                    .lbhomedir
                    .join("webfrontend/htmlauth/plugins")
                    .join(&p.folder);
                let has_web_ui = webui_dir.exists();

                // Format install date
                let install_date = p
                    .epoch_firstinstalled
                    .map(|ts| {
                        use std::time::{Duration, UNIX_EPOCH};
                        let d = UNIX_EPOCH + Duration::from_secs(ts);
                        format!("{:?}", d) // Simple formatting
                    })
                    .unwrap_or_else(|| "Unknown".to_string());

                displays.push(PluginDisplay {
                    md5: p.md5.clone(),
                    name: p.name.clone(),
                    folder: p.folder.clone(),
                    version: p.version.clone(),
                    author: p.author_name.clone(),
                    author_email: p.author_email.clone(),
                    title: p.title.get("en").cloned().unwrap_or_else(|| p.name.clone()),
                    has_web_ui,
                    has_daemon,
                    daemon_running,
                    install_date,
                });
            }
            displays
        }
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
pub async fn install_form(State(state): State<AppState>) -> Html<String> {
    let template = PluginInstallTemplate {
        version: state.version.clone(),
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
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
        Ok(Some(p)) => {
            // Check features
            let daemon_dir = state.lbhomedir.join("bin/plugins").join(&p.folder).join("daemon");
            let has_daemon = daemon_dir.exists();
            let webui_dir = state
                .lbhomedir
                .join("webfrontend/htmlauth/plugins")
                .join(&p.folder);
            let has_web_ui = webui_dir.exists();

            let install_date = p
                .epoch_firstinstalled
                .map(|ts| {
                    use std::time::{Duration, UNIX_EPOCH};
                    let d = UNIX_EPOCH + Duration::from_secs(ts);
                    format!("{:?}", d)
                })
                .unwrap_or_else(|| "Unknown".to_string());

            let plugin_display = PluginDisplay {
                md5: p.md5.clone(),
                name: p.name.clone(),
                folder: p.folder.clone(),
                version: p.version.clone(),
                author: p.author_name.clone(),
                author_email: p.author_email.clone(),
                title: p.title.get("en").cloned().unwrap_or_else(|| p.name.clone()),
                has_web_ui,
                has_daemon,
                daemon_running: is_daemon_running(&state.lbhomedir, &p.folder),
                install_date,
            };

            let template = PluginDetailsTemplate {
                plugin: plugin_display,
                version: state.version.clone(),
            };

            Html(
                template
                    .render()
                    .unwrap_or_else(|_| "Error rendering template".to_string()),
            )
        }
        _ => Html(String::from(
            "<html><body><h1>Plugin not found</h1><a href='/plugins'>Back to plugins</a></body></html>",
        )),
    }
}

/// Check whether a plugin daemon is running by inspecting its pidfile.
///
/// Looks for `$LBHOMEDIR/run/plugins/{folder}/{folder}.pid` and verifies
/// the recorded PID exists in `/proc`.
fn is_daemon_running(lbhomedir: &std::path::Path, folder: &str) -> bool {
    // Primary: pidfile at run/plugins/{folder}/{folder}.pid
    let pidfile = lbhomedir
        .join("run/plugins")
        .join(folder)
        .join(format!("{}.pid", folder));

    if let Ok(content) = std::fs::read_to_string(&pidfile) {
        let pid_str = content.trim();
        if let Ok(pid) = pid_str.parse::<u32>() {
            // On Linux, check /proc/{pid}
            return std::path::Path::new(&format!("/proc/{}", pid)).exists();
        }
    }

    // Fallback: check for any .pid file in the plugin's bin directory
    let bin_dir = lbhomedir.join("bin/plugins").join(folder);
    if let Ok(entries) = std::fs::read_dir(&bin_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("pid") {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(pid) = content.trim().parse::<u32>() {
                        return std::path::Path::new(&format!("/proc/{}", pid)).exists();
                    }
                }
            }
        }
    }

    false
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
