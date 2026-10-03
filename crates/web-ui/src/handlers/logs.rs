//! Log viewer UI handlers

use askama::Template;
use axum::{
    extract::{Query, State},
    response::Html,
};
use rustylox_logging::get_log_files;
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
    pub lang: String,
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

    let lang = state.config.read().await.base.lang.clone();
    let template = LogsTemplate {
        log_files,
        version: state.version.clone(),
        lang,
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

/// LoxBerry-compatible logfile.cgi handler
/// Handles: /admin/system/tools/logfile.cgi?logfile=/plugins/Foo/bar.log&header=html&format=template
pub async fn logfile_compat(
    State(state): State<AppState>,
    Query(query): Query<LogfileCompatQuery>,
) -> Html<String> {
    let logfile = match query.logfile {
        Some(ref f) if !f.is_empty() => f,
        _ => return Html("<p>No logfile specified.</p>".to_string()),
    };

    // Strip leading slash, prevent path traversal
    let rel = logfile.trim_start_matches('/');
    if rel.contains("..") {
        return Html("<p>Invalid path.</p>".to_string());
    }

    let log_dir = state.lbhomedir.join("log");
    let path = log_dir.join(rel);

    // Ensure resolved path stays within log dir
    let canonical_log = match log_dir.canonicalize() {
        Ok(p) => p,
        Err(_) => return Html("<p>Log directory not found.</p>".to_string()),
    };
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return Html(log_not_found_html(rel)),
    };
    if !canonical_path.starts_with(&canonical_log) {
        return Html("<p>Access denied.</p>".to_string());
    }

    match tokio::fs::read_to_string(&canonical_path).await {
        Ok(content) => {
            let file_name = canonical_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(rel);
            let escaped: String = content
                .lines()
                .map(|l| {
                    l.replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;")
                })
                .collect::<Vec<_>>()
                .join("\n");
            Html(format!(
                "<!DOCTYPE html><html><head>\
                 <meta charset='utf-8'>\
                 <title>{}</title>\
                 <link rel='stylesheet' href='/static/css/style.css'>\
                 </head><body style='background:#111;color:#d4d4d4;padding:16px;'>\
                 <h2 style='color:#eee;'>{}</h2>\
                 <pre style='background:#1e1e1e;padding:16px;border-radius:4px;\
                 overflow:auto;font-size:12px;white-space:pre-wrap;'>{}</pre>\
                 </body></html>",
                file_name, file_name, escaped
            ))
        }
        Err(e) => Html(format!("<p>Failed to read log: {}</p>", e)),
    }
}

#[derive(Debug, Deserialize)]
pub struct LogfileCompatQuery {
    pub logfile: Option<String>,
    // header and format params are accepted but ignored — we always render HTML
    pub header: Option<String>,
    pub format: Option<String>,
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
        Ok(content) => Html(render_log_view(
            file_name,
            &content,
            query.lines,
            query.search.as_deref().unwrap_or(""),
        )),
        Err(e) => Html(format!(
            "<div class='error'>Failed to read log: {}</div>",
            e
        )),
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn log_not_found_html(rel: &str) -> String {
    format!("<p>Log file not found: {}</p>", html_escape(rel))
}

/// Byte range in `line` of the first case-insensitive occurrence of
/// `term_lower` (which must already be lowercase and non-empty).
fn find_ignoring_case(line: &str, term_lower: &str) -> Option<(usize, usize)> {
    // Lowercasing can change byte lengths, so offsets in the lowercased text
    // are not offsets in `line`: remember which original char each
    // lowercased byte came from.
    let mut lower = String::with_capacity(line.len());
    let mut origin = Vec::with_capacity(line.len());
    for (idx, ch) in line.char_indices() {
        for lowered in ch.to_lowercase() {
            lower.push(lowered);
            origin.resize(lower.len(), idx);
        }
    }
    let pos = lower.find(term_lower)?;
    let start = *origin.get(pos)?;
    let last = *origin.get(pos + term_lower.len().checked_sub(1)?)?;
    let last_len = line.get(last..)?.chars().next()?.len_utf8();
    Some((start, last + last_len))
}

/// One log line as HTML, with the first occurrence of `search_term`
/// (already lowercased) highlighted.
fn render_log_line(line: &str, search_term: &str) -> String {
    // Match on the raw line and escape the pieces afterwards, so a match can
    // never start or end inside an HTML entity.
    let found = find_ignoring_case(line, search_term).and_then(|(start, end)| {
        Some((line.get(..start)?, line.get(start..end)?, line.get(end..)?))
    });
    match found {
        Some((before, matched, after)) => format!(
            "{}<mark style='background:#ff0;color:#000'>{}</mark>{}",
            html_escape(before),
            html_escape(matched),
            html_escape(after)
        ),
        None => html_escape(line),
    }
}

/// The HTML fragment for the log viewer: the last `lines` lines of
/// `content`, optionally narrowed to those containing `search`.
fn render_log_view(file_name: &str, content: &str, lines: usize, search: &str) -> String {
    let all_lines: Vec<&str> = content.lines().collect();
    let start = all_lines.len().saturating_sub(lines);
    let tail_lines = &all_lines[start..];

    // Apply search filter if provided
    let search_term = search.to_lowercase();
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
        .map(|l| render_log_line(l, &search_term))
        .collect::<Vec<_>>()
        .join("\n");

    let filter_info = if !search_term.is_empty() {
        format!(
            " &mdash; <span style='color:#aaa;'>{} matching \"{}\"</span>",
            filtered.len(),
            html_escape(&search_term)
        )
    } else {
        String::new()
    };

    format!(
        "<div style='display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;'>\
         <strong>{}</strong>\
         <span style='color: #888;'>{} lines{}</span></div>\
         <pre style='background: #1e1e1e; color: #d4d4d4; padding: 16px; border-radius: 4px; \
         overflow-x: auto; font-size: 12px; max-height: 600px; overflow-y: auto;'>{}</pre>",
        html_escape(file_name),
        filtered.len(),
        filter_info,
        escaped
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARK_OPEN: &str = "<mark style='background:#ff0;color:#000'>";

    #[test]
    fn search_term_is_highlighted_ignoring_case() {
        assert_eq!(
            render_log_line("an ERROR here", "error"),
            format!("an {MARK_OPEN}ERROR</mark> here")
        );
    }

    /// Lowercasing can change a string's byte length ("İ" grows by one
    /// byte), which used to shift the highlight - or panic when the shifted
    /// offset landed inside a multi-byte character.
    #[test]
    fn highlight_stays_on_the_match_when_lowercasing_changes_byte_lengths() {
        assert_eq!(render_log_line("İ€", "€"), format!("İ{MARK_OPEN}€</mark>"));
        assert_eq!(
            render_log_line("İstanbul error", "error"),
            format!("İstanbul {MARK_OPEN}error</mark>")
        );
    }

    #[test]
    fn highlight_never_lands_inside_an_html_entity() {
        assert_eq!(render_log_line("a < b", "lt"), "a &lt; b");
        assert_eq!(
            render_log_line("a < b", "<"),
            format!("a {MARK_OPEN}&lt;</mark> b")
        );
    }

    #[test]
    fn search_term_is_escaped_in_the_summary() {
        let html = render_log_view("a.log", "hello\n", 100, "<script>alert(1)</script>");

        assert!(!html.contains("<script>"), "{html}");
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn file_name_is_escaped_in_the_heading() {
        let html = render_log_view("<img src=x>.log", "hello\n", 100, "");

        assert!(!html.contains("<img"), "{html}");
    }

    #[test]
    fn missing_logfile_message_escapes_the_requested_path() {
        let html = log_not_found_html("<script>alert(1)</script>");

        assert!(!html.contains("<script>"), "{html}");
    }

    #[test]
    fn view_shows_only_the_requested_number_of_trailing_lines() {
        let html = render_log_view("a.log", "one\ntwo\nthree\n", 2, "");

        assert!(!html.contains("one"));
        assert!(html.contains("two\nthree"));
        assert!(html.contains("2 lines"));
    }
}
