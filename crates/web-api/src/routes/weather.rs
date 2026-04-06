//! Weather API routes
//!
//! Provides REST endpoints for weather data and the Loxone Cloud Emulator
//! endpoint on port 6066.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::AppState;

// ─── JSON response types ──────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct WeatherStatusResponse {
    pub enabled: bool,
    pub location: Option<String>,
    pub last_updated: Option<i64>,
    pub has_data: bool,
}

// ─── REST handlers ────────────────────────────────────────────────────────────

/// GET /api/weather/status – is the service enabled and when was data last fetched?
pub async fn status(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(svc) = &state.weather_service {
        let cfg = svc.config.read().await;
        let data = svc.data.read().await;
        Json(WeatherStatusResponse {
            enabled: cfg.enabled,
            location: if cfg.location_name.is_empty() {
                None
            } else {
                Some(cfg.location_name.clone())
            },
            last_updated: data.as_ref().map(|d| d.fetched_at),
            has_data: data.is_some(),
        })
        .into_response()
    } else {
        Json(WeatherStatusResponse {
            enabled: false,
            location: None,
            last_updated: None,
            has_data: false,
        })
        .into_response()
    }
}

/// GET /api/weather/current – current weather observation
pub async fn current(State(state): State<AppState>) -> Response {
    match &state.weather_service {
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Weather service not initialised",
        )
            .into_response(),
        Some(svc) => {
            let data = svc.data.read().await;
            match data.as_ref() {
                None => (StatusCode::NO_CONTENT, "No weather data yet").into_response(),
                Some(d) => Json(&d.current).into_response(),
            }
        }
    }
}

/// GET /api/weather/forecast – 7-day daily forecast
pub async fn forecast(State(state): State<AppState>) -> Response {
    match &state.weather_service {
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Weather service not initialised",
        )
            .into_response(),
        Some(svc) => {
            let data = svc.data.read().await;
            match data.as_ref() {
                None => (StatusCode::NO_CONTENT, "No weather data yet").into_response(),
                Some(d) => Json(&d.daily).into_response(),
            }
        }
    }
}

/// GET /api/weather/hourly – 168-hour hourly forecast
pub async fn hourly(State(state): State<AppState>) -> Response {
    match &state.weather_service {
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Weather service not initialised",
        )
            .into_response(),
        Some(svc) => {
            let data = svc.data.read().await;
            match data.as_ref() {
                None => (StatusCode::NO_CONTENT, "No weather data yet").into_response(),
                Some(d) => Json(&d.hourly).into_response(),
            }
        }
    }
}

/// GET /api/weather/all – complete weather dataset
pub async fn all(State(state): State<AppState>) -> Response {
    match &state.weather_service {
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Weather service not initialised",
        )
            .into_response(),
        Some(svc) => {
            let data = svc.data.read().await;
            match data.as_ref() {
                None => (StatusCode::NO_CONTENT, "No weather data yet").into_response(),
                Some(d) => Json(d.clone()).into_response(),
            }
        }
    }
}

/// GET /api/weather/config – current weather configuration
pub async fn get_config(State(state): State<AppState>) -> Response {
    match &state.weather_service {
        None => {
            // Return config from main AppState config
            let cfg = state.config.read().await;
            Json(cfg.weather.clone()).into_response()
        }
        Some(svc) => {
            let cfg = svc.config.read().await;
            Json(cfg.clone()).into_response()
        }
    }
}

/// PUT /api/weather/config – update weather configuration
pub async fn update_config(
    State(state): State<AppState>,
    Json(new_cfg): Json<rustylox_config::WeatherConfig>,
) -> Response {
    // Save to GeneralConfig
    {
        let mut config = state.config.write().await;
        config.weather = new_cfg.clone();
    }
    if let Err(e) = state
        .config_manager
        .save_general(&*state.config.read().await)
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save config: {}", e),
        )
            .into_response();
    }

    // Update live service
    if let Some(svc) = &state.weather_service {
        svc.update_config(new_cfg).await;
    }

    (StatusCode::OK, "Weather configuration updated").into_response()
}

/// POST /api/weather/refresh – trigger an immediate refresh
pub async fn refresh(State(state): State<AppState>) -> Response {
    match &state.weather_service {
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Weather service not initialised",
        )
            .into_response(),
        Some(svc) => match svc.refresh().await {
            Ok(()) => (StatusCode::OK, "Weather data refreshed").into_response(),
            Err(e) => (StatusCode::BAD_GATEWAY, format!("Refresh failed: {}", e)).into_response(),
        },
    }
}

// ─── Loxone Cloud Emulator ────────────────────────────────────────────────────
//
// The Loxone Miniserver polls:
//   http://weather.loxone.com:6066/forecast/?user=...&coord=LON,LAT&format=1&asl=ELEV
//
// We serve this endpoint on the main port as well (port 6066 is handled
// by the daemon which starts a second listener bound to that port).

#[derive(Deserialize)]
pub struct ForecastQuery {
    #[allow(dead_code)]
    pub user: Option<String>,
    pub coord: Option<String>,
    pub format: Option<u8>,
    pub asl: Option<f64>,
}

/// GET /forecast/ – Loxone Cloud Emulator endpoint
///
/// This mimics `http://weather.loxone.com:6066/forecast/`
pub async fn loxone_forecast(
    State(state): State<AppState>,
    Query(params): Query<ForecastQuery>,
) -> Response {
    info!("loxone_forecast: handler entered, coord={:?}", params.coord);

    match &state.weather_service {
        None => {
            warn!("loxone_forecast: weather_service is None");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "Weather service not initialised",
            )
                .into_response()
        }
        Some(svc) => {
            info!("loxone_forecast: weather_service is Some");

            // If coord query param is set, honour it (override config)
            if let Some(coord) = &params.coord {
                let parts: Vec<&str> = coord.split(',').collect();
                if parts.len() == 2 {
                    if let (Ok(lon), Ok(lat)) = (
                        parts[0].trim().parse::<f64>(),
                        parts[1].trim().parse::<f64>(),
                    ) {
                        debug!("loxone_forecast: acquiring config write lock");
                        let mut cfg = svc.config.write().await;
                        debug!("loxone_forecast: acquired config write lock");
                        if cfg.latitude == 0.0 || cfg.longitude == 0.0 {
                            cfg.latitude = lat;
                            cfg.longitude = lon;
                        }
                        if let Some(elev) = params.asl {
                            if cfg.elevation == 0.0 {
                                cfg.elevation = elev;
                            }
                        }
                    }
                }
            }

            // Ensure we have data
            debug!("loxone_forecast: acquiring data read lock (1)");
            {
                let data = svc.data.read().await;
                debug!("loxone_forecast: data is_none={}", data.is_none());
                if data.is_none() {
                    drop(data);
                    info!("loxone_forecast: no data, calling refresh()");
                    if let Err(e) = svc.refresh().await {
                        warn!("loxone_forecast: refresh failed: {}", e);
                        return (
                            StatusCode::BAD_GATEWAY,
                            format!("Weather fetch failed: {}", e),
                        )
                            .into_response();
                    }
                    info!("loxone_forecast: refresh() done");
                }
            }

            debug!("loxone_forecast: acquiring data read lock (2)");
            let data_guard = svc.data.read().await;
            debug!("loxone_forecast: got data guard");
            match data_guard.as_ref() {
                None => {
                    warn!("loxone_forecast: still no data after refresh");
                    (StatusCode::NO_CONTENT, "No weather data available").into_response()
                }
                Some(data) => {
                    info!(
                        "loxone_forecast: building response, hourly rows={}",
                        data.hourly.len()
                    );
                    let body = svc.loxone_emu_response(data);
                    info!("loxone_forecast: body built, {} bytes", body.len());
                    (
                        StatusCode::OK,
                        [(
                            axum::http::header::CONTENT_TYPE,
                            "text/plain; charset=utf-8",
                        )],
                        body,
                    )
                        .into_response()
                }
            }
        }
    }
}
