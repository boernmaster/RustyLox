//! MQTT subscription and conversion management handlers

use axum::{
    extract::{Path, State},
    response::Html,
    Form,
};
use serde::Deserialize;
use web_api::AppState;

#[derive(Debug, Deserialize)]
pub struct SubscriptionForm {
    pub topic: String,
    pub name: String,
    pub enabled: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConversionForm {
    pub topic_pattern: String,
    pub conversion_type: String,
    pub config: String,
    pub enabled: Option<String>,
}

// Escape HTML special characters to prevent XSS
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn conversion_type_label(t: &str) -> &str {
    match t {
        "bool_to_int" => "Boolean to 0/1",
        "json_expand" => "Expand JSON",
        "json_extract" => "Extract JSON Field",
        "regex" => "Regex Replace",
        _ => t,
    }
}

/// Render a single conversion item as read-only HTML
fn render_conversion_item_html(idx: usize, conv: &ParsedConversion) -> String {
    let enabled_class = if conv.enabled {
        "conversion-item"
    } else {
        "conversion-item disabled"
    };
    let type_label = conversion_type_label(&conv.conversion_type);
    let config_html = if !conv.config.is_empty() {
        format!(
            "<br><small style='color: #666;'>Config: {}</small>",
            html_escape(&conv.config)
        )
    } else {
        String::new()
    };
    let status_badge = if conv.enabled {
        "<span class='badge badge-success'>Active</span>"
    } else {
        "<span class='badge badge-warning'>Disabled</span>"
    };
    // Use r##"..."## so that "# inside hx-target="#..." does not terminate the raw string
    format!(
        r##"<div class="{enabled_class}" id="conversion-item-{idx}">
    <div style="display: flex; justify-content: space-between; align-items: center;">
        <div>
            <strong>{type_label}</strong>
            <br><code style="color: #1976D2;">{topic}</code>
            {config_html}
            {status_badge}
        </div>
        <div style="display: flex; gap: 8px;">
            <button class="btn btn-secondary btn-sm"
                    hx-get="/mqtt/conversions/{idx}/edit"
                    hx-target="#conversion-item-{idx}"
                    hx-swap="outerHTML">
                Edit
            </button>
            <button class="btn btn-danger btn-sm"
                    hx-delete="/mqtt/conversions/{idx}"
                    hx-target="#conversion-item-{idx}"
                    hx-swap="outerHTML">
                Delete
            </button>
        </div>
    </div>
</div>"##,
        enabled_class = enabled_class,
        idx = idx,
        type_label = html_escape(type_label),
        topic = html_escape(&conv.topic_pattern),
        config_html = config_html,
        status_badge = status_badge,
    )
}

/// Render an inline edit form for a conversion item
fn render_edit_form_html(idx: usize, conv: &ParsedConversion) -> String {
    let s_bool = if conv.conversion_type == "bool_to_int" {
        "selected"
    } else {
        ""
    };
    let s_expand = if conv.conversion_type == "json_expand" {
        "selected"
    } else {
        ""
    };
    let s_extract = if conv.conversion_type == "json_extract" {
        "selected"
    } else {
        ""
    };
    let s_regex = if conv.conversion_type == "regex" {
        "selected"
    } else {
        ""
    };
    let checked = if conv.enabled { "checked" } else { "" };
    // Use r##"..."## so that "# inside hx-target="#..." does not terminate the raw string
    format!(
        r##"<div class="conversion-item" id="conversion-item-{idx}">
    <form hx-put="/mqtt/conversions/{idx}"
          hx-target="#conversion-item-{idx}"
          hx-swap="outerHTML"
          style="width: 100%;">
        <div style="display: grid; grid-template-columns: 1fr auto; gap: 16px; align-items: start;">
            <div>
                <div class="form-group">
                    <label>Topic Pattern *</label>
                    <input type="text" name="topic_pattern" value="{topic}"
                           required placeholder="home/sensor/+" style="width: 100%;">
                </div>
                <div class="form-group">
                    <label>Conversion Type *</label>
                    <select name="conversion_type" required style="width: 100%;"
                            onchange="updateConvTypeHint(this)">
                        <option value="bool_to_int" {s_bool}>Boolean to 0/1</option>
                        <option value="json_expand" {s_expand}>Expand JSON</option>
                        <option value="json_extract" {s_extract}>Extract JSON Field</option>
                        <option value="regex" {s_regex}>Regex Replace</option>
                    </select>
                </div>
                <div class="conv-hint-box" style="margin-bottom: 10px;"></div>
                <div class="form-group">
                    <label>Configuration (JSON)</label>
                    <textarea name="config" rows="2" style="width: 100%;"
                              placeholder="Optional JSON config">{config}</textarea>
                </div>
                <div class="form-group">
                    <label><input type="checkbox" name="enabled" {checked}> Enabled</label>
                </div>
            </div>
            <div style="display: flex; flex-direction: column; gap: 8px; padding-top: 4px;">
                <button type="submit" class="btn btn-primary btn-sm">Save</button>
                <button type="button" class="btn btn-secondary btn-sm"
                        hx-get="/mqtt/conversions/{idx}/view"
                        hx-target="#conversion-item-{idx}"
                        hx-swap="outerHTML">Cancel</button>
            </div>
        </div>
    </form>
</div>"##,
        idx = idx,
        topic = html_escape(&conv.topic_pattern),
        s_bool = s_bool,
        s_expand = s_expand,
        s_extract = s_extract,
        s_regex = s_regex,
        config = html_escape(&conv.config),
        checked = checked,
    )
}

/// Render full list HTML (used by list, add, and update handlers)
fn render_full_list(conversions: &[ParsedConversion]) -> String {
    if conversions.is_empty() {
        return "<p style='text-align: center; color: #999; padding: 20px;'>No conversions configured</p>".to_string();
    }
    conversions
        .iter()
        .enumerate()
        .map(|(idx, conv)| render_conversion_item_html(idx, conv))
        .collect::<Vec<_>>()
        .join("\n")
}

/// List all MQTT subscriptions
pub async fn list_subscriptions(State(state): State<AppState>) -> Html<String> {
    let config_path = state.lbhomedir.join("config/system/mqtt_subscriptions.cfg");

    let subscriptions = match tokio::fs::read_to_string(&config_path).await {
        Ok(content) => parse_subscriptions_cfg(&content),
        Err(_) => Vec::new(),
    };

    let mut html = String::new();

    if subscriptions.is_empty() {
        html.push_str("<p style='text-align: center; color: #999; padding: 20px;'>No subscriptions configured</p>");
    } else {
        for (idx, sub) in subscriptions.iter().enumerate() {
            let enabled_class = if sub.enabled {
                "subscription-item"
            } else {
                "subscription-item disabled"
            };

            html.push_str(&format!(
                r#"<div class="{}">
                    <div style="display: flex; justify-content: space-between; align-items: center;">
                        <div>
                            <strong>{}</strong>
                            <br><code style="color: #1976D2;">{}</code>
                            {}
                        </div>
                        <div>
                            <button class="btn btn-danger btn-sm"
                                    hx-delete="/mqtt/subscriptions/{}"
                                    hx-target="closest .subscription-item"
                                    hx-swap="outerHTML">
                                Delete
                            </button>
                        </div>
                    </div>
                </div>"#,
                enabled_class,
                sub.name,
                sub.topic,
                if sub.enabled {
                    "<span class='badge badge-success'>Active</span>"
                } else {
                    "<span class='badge badge-warning'>Disabled</span>"
                },
                idx
            ));
        }
    }

    Html(html)
}

/// Add a new subscription
pub async fn add_subscription(
    State(state): State<AppState>,
    Form(form): Form<SubscriptionForm>,
) -> Html<String> {
    let config_path = state.lbhomedir.join("config/system/mqtt_subscriptions.cfg");

    let mut content = tokio::fs::read_to_string(&config_path)
        .await
        .unwrap_or_default();

    let section_name = form
        .name
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>();

    content.push_str(&format!(
        "\n[{}]\nTOPIC={}\nNAME={}\nENABLED={}\n",
        section_name,
        form.topic,
        form.name,
        if form.enabled.is_some() { "1" } else { "0" }
    ));

    if let Err(e) = tokio::fs::write(&config_path, content).await {
        return Html(format!(
            "<div class='alert alert-danger'>Error saving subscription: {}</div>",
            e
        ));
    }

    let enabled = form.enabled.is_some();
    let enabled_class = if enabled {
        "subscription-item"
    } else {
        "subscription-item disabled"
    };

    Html(format!(
        r#"<div class="{}">
            <div style="display: flex; justify-content: space-between; align-items: center;">
                <div>
                    <strong>{}</strong>
                    <br><code style="color: #1976D2;">{}</code>
                    {}
                </div>
                <div>
                    <button class="btn btn-danger btn-sm"
                            hx-delete="/mqtt/subscriptions/delete"
                            hx-target="closest .subscription-item"
                            hx-swap="outerHTML">
                        Delete
                    </button>
                </div>
            </div>
        </div>"#,
        enabled_class,
        form.name,
        form.topic,
        if enabled {
            "<span class='badge badge-success'>Active</span>"
        } else {
            "<span class='badge badge-warning'>Disabled</span>"
        }
    ))
}

/// Delete a subscription
pub async fn delete_subscription(
    State(state): State<AppState>,
    Path(idx): Path<usize>,
) -> Html<String> {
    let config_path = state.lbhomedir.join("config/system/mqtt_subscriptions.cfg");

    let content = match tokio::fs::read_to_string(&config_path).await {
        Ok(c) => c,
        Err(_) => return Html(String::new()),
    };

    let mut subscriptions = parse_subscriptions_cfg(&content);

    if idx < subscriptions.len() {
        subscriptions.remove(idx);
    }

    let mut new_content = String::new();
    for (i, sub) in subscriptions.iter().enumerate() {
        let section_name = format!("Subscription{}", i + 1);

        new_content.push_str(&format!(
            "[{}]\nTOPIC={}\nNAME={}\nENABLED={}\n\n",
            section_name,
            sub.topic,
            sub.name,
            if sub.enabled { "1" } else { "0" }
        ));
    }

    let _ = tokio::fs::write(&config_path, new_content).await;

    Html(String::new()) // Empty response will remove the element
}

/// List all conversions
pub async fn list_conversions(State(state): State<AppState>) -> Html<String> {
    let config_path = state.lbhomedir.join("config/system/mqtt_transformers.cfg");

    let conversions = match tokio::fs::read_to_string(&config_path).await {
        Ok(content) => parse_conversions_cfg(&content),
        Err(_) => Vec::new(),
    };

    Html(render_full_list(&conversions))
}

/// Return the read-only view for a single conversion (used by Cancel in edit form)
pub async fn get_conversion_view(
    State(state): State<AppState>,
    Path(idx): Path<usize>,
) -> Html<String> {
    let config_path = state.lbhomedir.join("config/system/mqtt_transformers.cfg");

    let conversions = match tokio::fs::read_to_string(&config_path).await {
        Ok(content) => parse_conversions_cfg(&content),
        Err(_) => Vec::new(),
    };

    if let Some(conv) = conversions.get(idx) {
        Html(render_conversion_item_html(idx, conv))
    } else {
        Html(String::new())
    }
}

/// Return the inline edit form for a conversion
pub async fn get_edit_conversion(
    State(state): State<AppState>,
    Path(idx): Path<usize>,
) -> Html<String> {
    let config_path = state.lbhomedir.join("config/system/mqtt_transformers.cfg");

    let conversions = match tokio::fs::read_to_string(&config_path).await {
        Ok(content) => parse_conversions_cfg(&content),
        Err(_) => Vec::new(),
    };

    if let Some(conv) = conversions.get(idx) {
        Html(render_edit_form_html(idx, conv))
    } else {
        Html(format!(
            "<div class='alert alert-danger'>Conversion {} not found</div>",
            idx
        ))
    }
}

/// Update an existing conversion (PUT handler)
pub async fn update_conversion(
    State(state): State<AppState>,
    Path(idx): Path<usize>,
    Form(form): Form<ConversionForm>,
) -> Html<String> {
    let config_path = state.lbhomedir.join("config/system/mqtt_transformers.cfg");

    let content = match tokio::fs::read_to_string(&config_path).await {
        Ok(c) => c,
        Err(_) => String::new(),
    };

    let mut conversions = parse_conversions_cfg(&content);

    if idx >= conversions.len() {
        return Html(format!(
            "<div class='alert alert-danger'>Conversion {} not found</div>",
            idx
        ));
    }

    conversions[idx] = ParsedConversion {
        topic_pattern: form.topic_pattern.clone(),
        conversion_type: form.conversion_type.clone(),
        config: form.config.clone(),
        enabled: form.enabled.is_some(),
    };

    let mut new_content = String::new();
    for (i, conv) in conversions.iter().enumerate() {
        let section_name = format!("Conversion{}", i + 1);
        new_content.push_str(&format!(
            "[{}]\nTOPIC_PATTERN={}\nTYPE={}\nCONFIG={}\nENABLED={}\n\n",
            section_name,
            conv.topic_pattern,
            conv.conversion_type,
            conv.config,
            if conv.enabled { "1" } else { "0" }
        ));
    }

    if let Err(e) = tokio::fs::write(&config_path, new_content).await {
        return Html(format!(
            "<div class='alert alert-danger'>Error saving conversion: {}</div>",
            e
        ));
    }

    Html(render_conversion_item_html(idx, &conversions[idx]))
}

/// Add a new conversion and return the full refreshed list
pub async fn add_conversion(
    State(state): State<AppState>,
    Form(form): Form<ConversionForm>,
) -> Html<String> {
    let config_path = state.lbhomedir.join("config/system/mqtt_transformers.cfg");

    let mut content = tokio::fs::read_to_string(&config_path)
        .await
        .unwrap_or_default();

    let section_name = format!(
        "Conversion{}",
        parse_conversions_cfg(&content).len() + 1
    );

    content.push_str(&format!(
        "\n[{}]\nTOPIC_PATTERN={}\nTYPE={}\nCONFIG={}\nENABLED={}\n",
        section_name,
        form.topic_pattern,
        form.conversion_type,
        form.config,
        if form.enabled.is_some() { "1" } else { "0" }
    ));

    if let Err(e) = tokio::fs::write(&config_path, &content).await {
        return Html(format!(
            "<div class='alert alert-danger'>Error saving conversion: {}</div>",
            e
        ));
    }

    // Return the full refreshed list so indices are always correct
    let conversions = parse_conversions_cfg(&content);
    Html(render_full_list(&conversions))
}

/// Delete a conversion
pub async fn delete_conversion(
    State(state): State<AppState>,
    Path(idx): Path<usize>,
) -> Html<String> {
    let config_path = state.lbhomedir.join("config/system/mqtt_transformers.cfg");

    let content = match tokio::fs::read_to_string(&config_path).await {
        Ok(c) => c,
        Err(_) => return Html(String::new()),
    };

    let mut conversions = parse_conversions_cfg(&content);

    if idx < conversions.len() {
        conversions.remove(idx);
    }

    let mut new_content = String::new();
    for (i, conv) in conversions.iter().enumerate() {
        let section_name = format!("Conversion{}", i + 1);
        new_content.push_str(&format!(
            "[{}]\nTOPIC_PATTERN={}\nTYPE={}\nCONFIG={}\nENABLED={}\n\n",
            section_name,
            conv.topic_pattern,
            conv.conversion_type,
            conv.config,
            if conv.enabled { "1" } else { "0" }
        ));
    }

    let _ = tokio::fs::write(&config_path, new_content).await;

    Html(String::new())
}

// Helper structs
#[derive(Debug, Clone)]
struct ParsedSubscription {
    topic: String,
    name: String,
    enabled: bool,
}

#[derive(Debug, Clone)]
struct ParsedConversion {
    topic_pattern: String,
    conversion_type: String,
    config: String,
    enabled: bool,
}

// Parser for INI-style subscription config
fn parse_subscriptions_cfg(content: &str) -> Vec<ParsedSubscription> {
    let mut subscriptions = Vec::new();
    let mut current_topic = String::new();
    let mut current_name = String::new();
    let mut current_enabled = true;

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') {
            if !current_topic.is_empty() {
                subscriptions.push(ParsedSubscription {
                    topic: current_topic.clone(),
                    name: current_name.clone(),
                    enabled: current_enabled,
                });
            }
            current_topic.clear();
            current_name.clear();
            current_enabled = true;
        } else if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "TOPIC" => current_topic = value.trim().to_string(),
                "NAME" => current_name = value.trim().to_string(),
                "ENABLED" => current_enabled = value.trim() == "1",
                _ => {}
            }
        }
    }

    if !current_topic.is_empty() {
        subscriptions.push(ParsedSubscription {
            topic: current_topic,
            name: current_name,
            enabled: current_enabled,
        });
    }

    subscriptions
}

// Parser for INI-style conversion config
fn parse_conversions_cfg(content: &str) -> Vec<ParsedConversion> {
    let mut conversions = Vec::new();
    let mut current_pattern = String::new();
    let mut current_type = String::new();
    let mut current_config = String::new();
    let mut current_enabled = true;

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') {
            if !current_pattern.is_empty() {
                conversions.push(ParsedConversion {
                    topic_pattern: current_pattern.clone(),
                    conversion_type: current_type.clone(),
                    config: current_config.clone(),
                    enabled: current_enabled,
                });
            }
            current_pattern.clear();
            current_type.clear();
            current_config.clear();
            current_enabled = true;
        } else if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "TOPIC_PATTERN" => current_pattern = value.trim().to_string(),
                "TYPE" => current_type = value.trim().to_string(),
                "CONFIG" => current_config = value.trim().to_string(),
                "ENABLED" => current_enabled = value.trim() == "1",
                _ => {}
            }
        }
    }

    if !current_pattern.is_empty() {
        conversions.push(ParsedConversion {
            topic_pattern: current_pattern,
            conversion_type: current_type,
            config: current_config,
            enabled: current_enabled,
        });
    }

    conversions
}
