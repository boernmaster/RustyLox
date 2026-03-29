//! Weather web UI handlers

use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    Form,
};
use serde::Deserialize;
use web_api::AppState;

use crate::templates::{WeatherConfigTemplate, WeatherIndexTemplate};

/// GET /weather – main weather page (current + 7-day forecast)
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let (weather_data, cfg) = if let Some(svc) = &state.weather_service {
        let data = svc.data.read().await.clone();
        let cfg = svc.config.read().await.clone();
        (data, cfg)
    } else {
        let cfg = state.config.read().await.weather.clone();
        (None, cfg)
    };

    let template = WeatherIndexTemplate {
        version: state.version.clone(),
        weather: weather_data,
        enabled: cfg.enabled,
        location_name: cfg.location_name.clone(),
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// GET /weather/config – weather configuration page
pub async fn config(State(state): State<AppState>) -> Html<String> {
    let cfg = if let Some(svc) = &state.weather_service {
        svc.config.read().await.clone()
    } else {
        state.config.read().await.weather.clone()
    };

    let template = WeatherConfigTemplate {
        version: state.version.clone(),
        cfg,
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

#[derive(Debug, Deserialize)]
pub struct WeatherConfigForm {
    #[serde(default)]
    pub enabled: Option<String>, // checkbox: "on" or absent
    pub latitude: String,
    pub longitude: String,
    pub location_name: String,
    pub elevation: String,
    pub update_interval_minutes: String,
    #[serde(default)]
    pub metric: Option<String>,
    #[serde(default)]
    pub push_udp: Option<String>,
    pub miniserver_key: String,
    pub miniserver_udp_port: String,
    #[serde(default)]
    pub cloud_emu: Option<String>,
    #[serde(default)]
    pub dnsmasq_enabled: Option<String>,
    pub local_ip: String,
    #[serde(default)]
    pub send_mqtt: Option<String>,
    pub mqtt_topic: String,
}

/// POST /weather/config – save weather configuration
pub async fn config_submit(
    State(state): State<AppState>,
    Form(form): Form<WeatherConfigForm>,
) -> impl IntoResponse {
    let new_cfg = rustylox_config::WeatherConfig {
        enabled: form.enabled.as_deref() == Some("on"),
        latitude: form.latitude.parse().unwrap_or(0.0),
        longitude: form.longitude.parse().unwrap_or(0.0),
        location_name: form.location_name.clone(),
        elevation: form.elevation.parse().unwrap_or(0.0),
        update_interval_minutes: form.update_interval_minutes.parse().unwrap_or(15),
        metric: form.metric.as_deref() == Some("on"),
        push_udp: form.push_udp.as_deref() == Some("on"),
        miniserver_key: form.miniserver_key.clone(),
        miniserver_udp_port: form.miniserver_udp_port.parse().unwrap_or(7044),
        cloud_emu: form.cloud_emu.as_deref() == Some("on"),
        dnsmasq_enabled: form.dnsmasq_enabled.as_deref() == Some("on"),
        local_ip: form.local_ip.clone(),
        miniserver_ip: String::new(), // runtime-only, re-resolved by daemon on next start
        send_mqtt: form.send_mqtt.as_deref() == Some("on"),
        mqtt_topic: form.mqtt_topic.clone(),
    };

    // Save to disk
    {
        let mut config = state.config.write().await;
        config.weather = new_cfg.clone();
    }

    if let Err(e) = state
        .config_manager
        .save_general(&*state.config.read().await)
        .await
    {
        return Html(format!(
            "<div class='alert alert-danger'>Error saving: {}</div>",
            e
        ));
    }

    // Update live service
    if let Some(svc) = &state.weather_service {
        svc.update_config(new_cfg).await;
    }

    Html(
        "<div class='alert alert-success'>Weather configuration saved. \
         Refresh data with the button on the Weather page.</div>"
            .to_string(),
    )
}

/// POST /weather/refresh – trigger immediate data refresh
pub async fn refresh(State(state): State<AppState>) -> Html<String> {
    match &state.weather_service {
        None => {
            Html("<div class='alert alert-danger'>Weather service not available.</div>".to_string())
        }
        Some(svc) => match svc.refresh().await {
            Ok(()) => Html(
                "<div class='alert alert-success'>Weather data refreshed successfully.</div>"
                    .to_string(),
            ),
            Err(e) => Html(format!(
                "<div class='alert alert-danger'>Refresh failed: {}</div>",
                e
            )),
        },
    }
}
