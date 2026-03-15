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

/// List all MQTT subscriptions
pub async fn list_subscriptions(State(state): State<AppState>) -> Html<String> {
    // Read subscriptions from config
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

    // Read existing subscriptions
    let mut content = tokio::fs::read_to_string(&config_path)
        .await
        .unwrap_or_default();

    // Generate section name from topic
    let section_name = form
        .name
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>();

    // Append new subscription
    content.push_str(&format!(
        "\n[{}]\nTOPIC={}\nNAME={}\nENABLED={}\n",
        section_name,
        form.topic,
        form.name,
        if form.enabled.is_some() { "1" } else { "0" }
    ));

    // Save
    if let Err(e) = tokio::fs::write(&config_path, content).await {
        return Html(format!(
            "<div class='alert alert-danger'>Error saving subscription: {}</div>",
            e
        ));
    }

    // Return new subscription HTML
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

    // Rebuild config file
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

    let mut html = String::new();

    if conversions.is_empty() {
        html.push_str("<p style='text-align: center; color: #999; padding: 20px;'>No conversions configured</p>");
    } else {
        for (idx, conv) in conversions.iter().enumerate() {
            let enabled_class = if conv.enabled {
                "conversion-item"
            } else {
                "conversion-item disabled"
            };

            html.push_str(&format!(
                r#"<div class="{}">
                    <div style="display: flex; justify-content: space-between; align-items: center;">
                        <div>
                            <strong>{}</strong>
                            <br><code style="color: #1976D2;">{}</code>
                            <br><small style="color: #666;">{}</small>
                            {}
                        </div>
                        <div>
                            <button class="btn btn-danger btn-sm"
                                    hx-delete="/mqtt/conversions/{}"
                                    hx-target="closest .conversion-item"
                                    hx-swap="outerHTML">
                                Delete
                            </button>
                        </div>
                    </div>
                </div>"#,
                enabled_class,
                conv.conversion_type,
                conv.topic_pattern,
                conv.config,
                if conv.enabled {
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

/// Add a new conversion
pub async fn add_conversion(
    State(state): State<AppState>,
    Form(form): Form<ConversionForm>,
) -> Html<String> {
    let config_path = state.lbhomedir.join("config/system/mqtt_transformers.cfg");

    let mut content = tokio::fs::read_to_string(&config_path)
        .await
        .unwrap_or_default();

    let section_name = form
        .conversion_type
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>();

    content.push_str(&format!(
        "\n[{}]\nTOPIC_PATTERN={}\nTYPE={}\nCONFIG={}\nENABLED={}\n",
        section_name,
        form.topic_pattern,
        form.conversion_type,
        form.config,
        if form.enabled.is_some() { "1" } else { "0" }
    ));

    if let Err(e) = tokio::fs::write(&config_path, content).await {
        return Html(format!(
            "<div class='alert alert-danger'>Error saving conversion: {}</div>",
            e
        ));
    }

    let enabled = form.enabled.is_some();
    let enabled_class = if enabled {
        "conversion-item"
    } else {
        "conversion-item disabled"
    };

    Html(format!(
        r#"<div class="{}">
            <div style="display: flex; justify-content: space-between; align-items: center;">
                <div>
                    <strong>{}</strong>
                    <br><code style="color: #1976D2;">{}</code>
                    <br><small style="color: #666;">{}</small>
                    {}
                </div>
                <div>
                    <button class="btn btn-danger btn-sm"
                            hx-delete="/mqtt/conversions/delete"
                            hx-target="closest .conversion-item"
                            hx-swap="outerHTML">
                        Delete
                    </button>
                </div>
            </div>
        </div>"#,
        enabled_class,
        form.conversion_type,
        form.topic_pattern,
        form.config,
        if enabled {
            "<span class='badge badge-success'>Active</span>"
        } else {
            "<span class='badge badge-warning'>Disabled</span>"
        }
    ))
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
            // New section - save previous if complete
            if !current_topic.is_empty() {
                subscriptions.push(ParsedSubscription {
                    topic: current_topic.clone(),
                    name: current_name.clone(),
                    enabled: current_enabled,
                });
            }
            // Reset for new section
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

    // Save last subscription
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
