//! Log viewer UI handlers

use askama::Template;
use axum::{
    extract::{Query, State},
    response::Html,
};
use loxberry_logging::get_log_files;
use serde::Deserialize;
use web_api::AppState;

#[derive(Debug)]
pub struct LogFileDisplay {
    pub name: String,
    pub size_human: String,
    pub modified: String,
}

#[derive(Template)]
#[template(path = "logs.html")]
pub struct LogsTemplate {
    pub log_files: Vec<LogFileDisplay>,
    pub version: String,
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Show log viewer page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let log_dir = state.lbhomedir.join("log/system");

    let log_files = match get_log_files(&log_dir) {
        Ok(files) => files
            .into_iter()
            .map(|f| LogFileDisplay {
                name: f.name,
                size_human: format_size(f.size),
                modified: f.modified.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            })
            .collect(),
        Err(e) => {
            tracing::error!("Failed to list log files: {}", e);
            Vec::new()
        }
    };

    let template = LogsTemplate {
        log_files,
        version: state.version.clone(),
    };
    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

#[derive(Debug, Deserialize)]
pub struct ViewLogQuery {
    pub file: Option<String>,
    #[serde(default = "default_lines")]
    pub lines: usize,
    pub search: Option<String>,
}

fn default_lines() -> usize {
    100
}

/// View log file contents (HTMX endpoint)
pub async fn view(
    State(state): State<AppState>,
    Query(query): Query<ViewLogQuery>,
) -> Html<String> {
    let file_name = match query.file {
        Some(ref f) if !f.is_empty() => f,
        _ => return Html("<p style='color: #888;'>No file selected.</p>".to_string()),
    };

    // Prevent path traversal
    if file_name.contains('/') || file_name.contains("..") {
        return Html("<div class='error'>Invalid file name.</div>".to_string());
    }

    let log_dir = state.lbhomedir.join("log/system");
    let path = log_dir.join(file_name);

    if !path.exists() {
        return Html("<div class='error'>Log file not found.</div>".to_string());
    }

    match tokio::fs::read_to_string(&path).await {
        Ok(content) => {
            let all_lines: Vec<&str> = content.lines().collect();
            let start = all_lines.len().saturating_sub(query.lines);
            let tail_lines = &all_lines[start..];

            // Apply search filter if provided
            let search_term = query.search.as_deref().unwrap_or("").to_lowercase();
            let filtered: Vec<&str> = if search_term.is_empty() {
                tail_lines.to_vec()
            } else {
                tail_lines
                    .iter()
                    .filter(|l| l.to_lowercase().contains(&search_term))
                    .copied()
                    .collect()
            };

            let escaped: String = filtered
                .iter()
                .map(|l| {
                    let escaped = l
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;");
                    // Highlight search term if present
                    if search_term.is_empty() {
                        escaped
                    } else {
                        // Case-insensitive highlight
                        let lower = escaped.to_lowercase();
                        if let Some(pos) = lower.find(&search_term) {
                            let end = pos + search_term.len();
                            format!(
                                "{}<mark style='background:#ff0;color:#000'>{}</mark>{}",
                                &escaped[..pos],
                                &escaped[pos..end],
                                &escaped[end..]
                            )
                        } else {
                            escaped
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            let filter_info = if !search_term.is_empty() {
                format!(
                    " &mdash; <span style='color:#aaa;'>{} matching \"{}\"</span>",
                    filtered.len(),
                    search_term
                )
            } else {
                String::new()
            };

            Html(format!(
                "<div style='display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;'>\
                 <strong>{}</strong>\
                 <span style='color: #888;'>{} lines{}</span></div>\
                 <pre style='background: #1e1e1e; color: #d4d4d4; padding: 16px; border-radius: 4px; \
                 overflow-x: auto; font-size: 12px; max-height: 600px; overflow-y: auto;'>{}</pre>",
                file_name,
                filtered.len(),
                filter_info,
                escaped
            ))
        }
        Err(e) => Html(format!(
            "<div class='error'>Failed to read log: {}</div>",
            e
        )),
    }
}
