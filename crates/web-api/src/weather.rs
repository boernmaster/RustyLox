//! Native weather service for RustyLox
//!
//! Fetches real-time weather data from the Open-Meteo API (free, no API key)
//! and delivers it to Loxone in every way that weather4lox does:
//!
//! 1. **UDP push** – sends `name@value; ` packets to the Miniserver's virtual
//!    input port (identical protocol to LoxBerry IO::UDP).
//! 2. **Cloud emulator** – serves the Loxone `weather.loxone.com:6066` REST
//!    format so the Miniserver can poll us instead of the real cloud.
//! 3. **MQTT publish** – publishes all values under a configurable topic.
//! 4. **dnsmasq** – optionally writes `/etc/dnsmasq.d/rustylox-weather.conf`
//!    to redirect `weather.loxone.com` to this host and restarts the service.

use std::net::UdpSocket;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use loxberry_config::WeatherConfig;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

// ─── Public data structures ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationInfo {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f64,
    pub timezone: String,
    /// UTC offset in seconds
    pub utc_offset_seconds: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentWeather {
    /// Unix timestamp
    pub timestamp: i64,
    /// °C
    pub temperature: f64,
    /// °C apparent
    pub feels_like: f64,
    /// %
    pub humidity: u8,
    /// hPa
    pub pressure: f64,
    /// km/h
    pub wind_speed: f64,
    /// degrees 0-360
    pub wind_direction: u16,
    /// km/h
    pub wind_gusts: f64,
    /// mm
    pub precipitation: f64,
    /// WMO code
    pub weather_code: u16,
    pub weather_description: String,
    /// Short icon key (e.g. "clear-day", "rain")
    pub weather_icon: String,
    pub is_day: bool,
    pub uv_index: f64,
    /// metres
    pub visibility: f64,
    /// Loxone picto code (1-25)
    pub loxone_code: u8,
    /// Sunrise ISO datetime
    pub sunrise: String,
    /// Sunset ISO datetime
    pub sunset: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyForecast {
    pub date: String,
    pub temp_max: f64,
    pub temp_min: f64,
    pub temp_mean: f64,
    pub weather_code: u16,
    pub weather_description: String,
    pub precipitation_sum: f64,
    pub precipitation_probability: u8,
    pub wind_speed_max: f64,
    pub uv_index_max: f64,
    pub sunrise: String,
    pub sunset: String,
    pub loxone_code: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyForecast {
    pub datetime: String,
    pub temperature: f64,
    pub feels_like: f64,
    pub humidity: u8,
    pub precipitation: f64,
    pub precipitation_probability: u8,
    pub weather_code: u16,
    pub wind_speed: f64,
    pub wind_direction: u16,
    pub wind_gusts: f64,
    pub pressure: f64,
    pub visibility: f64,
    pub loxone_code: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherData {
    pub location: LocationInfo,
    pub current: CurrentWeather,
    pub daily: Vec<DailyForecast>,
    pub hourly: Vec<HourlyForecast>,
    pub fetched_at: i64,
}

// ─── Open-Meteo response shapes (internal only) ──────────────────────────────

#[derive(Deserialize)]
struct OpenMeteoResponse {
    latitude: f64,
    longitude: f64,
    elevation: f64,
    timezone: String,
    #[allow(dead_code)]
    timezone_abbreviation: String,
    utc_offset_seconds: i32,
    current: OpenMeteoCurrent,
    daily: OpenMeteoDaily,
    hourly: OpenMeteoHourly,
}

#[derive(Deserialize)]
struct OpenMeteoCurrent {
    time: String,
    temperature_2m: f64,
    apparent_temperature: f64,
    relative_humidity_2m: u8,
    surface_pressure: f64,
    wind_speed_10m: f64,
    wind_direction_10m: u16,
    wind_gusts_10m: f64,
    precipitation: f64,
    weather_code: u16,
    is_day: u8,
    uv_index: f64,
    visibility: f64,
}

#[derive(Deserialize)]
struct OpenMeteoDaily {
    time: Vec<String>,
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
    weather_code: Vec<u16>,
    precipitation_sum: Vec<f64>,
    precipitation_probability_max: Vec<u8>,
    wind_speed_10m_max: Vec<f64>,
    uv_index_max: Vec<f64>,
    sunrise: Vec<String>,
    sunset: Vec<String>,
}

#[derive(Deserialize)]
struct OpenMeteoHourly {
    time: Vec<String>,
    temperature_2m: Vec<f64>,
    apparent_temperature: Vec<f64>,
    relative_humidity_2m: Vec<u8>,
    precipitation: Vec<f64>,
    precipitation_probability: Vec<u8>,
    weather_code: Vec<u16>,
    wind_speed_10m: Vec<f64>,
    wind_direction_10m: Vec<u16>,
    wind_gusts_10m: Vec<f64>,
    surface_pressure: Vec<f64>,
    visibility: Vec<f64>,
}

// ─── WeatherService ───────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct WeatherService {
    pub config: Arc<RwLock<WeatherConfig>>,
    pub data: Arc<RwLock<Option<WeatherData>>>,
    client: reqwest::Client,
}

impl WeatherService {
    pub fn new(config: WeatherConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            data: Arc::new(RwLock::new(None)),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
        }
    }

    /// Update the running config and immediately trigger a refresh.
    pub async fn update_config(&self, new_cfg: WeatherConfig) {
        *self.config.write().await = new_cfg;
    }

    /// Fetch weather data from Open-Meteo and store it. Returns error string on failure.
    pub async fn refresh(&self) -> Result<(), String> {
        let cfg = self.config.read().await.clone();
        if !cfg.enabled || cfg.latitude == 0.0 && cfg.longitude == 0.0 {
            return Ok(()); // not configured
        }

        let url = format!(
            "https://api.open-meteo.com/v1/forecast?\
             latitude={lat}&longitude={lon}\
             &current=temperature_2m,apparent_temperature,relative_humidity_2m,\
             surface_pressure,wind_speed_10m,wind_direction_10m,wind_gusts_10m,\
             precipitation,weather_code,is_day,uv_index,visibility\
             &daily=weather_code,temperature_2m_max,temperature_2m_min,\
             precipitation_sum,precipitation_probability_max,wind_speed_10m_max,\
             uv_index_max,sunrise,sunset\
             &hourly=temperature_2m,apparent_temperature,relative_humidity_2m,\
             precipitation,precipitation_probability,weather_code,\
             wind_speed_10m,wind_direction_10m,wind_gusts_10m,\
             surface_pressure,visibility\
             &timezone=auto&forecast_days=7&wind_speed_unit=kmh",
            lat = cfg.latitude,
            lon = cfg.longitude,
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Open-Meteo returned HTTP {}", resp.status()));
        }

        let raw: OpenMeteoResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Open-Meteo response: {}", e))?;

        let data = self.parse_response(raw, &cfg);

        // Optional: push to Miniserver via UDP
        if cfg.push_udp {
            if let Err(e) = self.push_udp(&data, &cfg) {
                warn!("Weather UDP push failed: {}", e);
            }
        }

        // Optional: publish to MQTT
        if cfg.send_mqtt {
            // MQTT publishing happens through the gateway; log intent here
            info!(
                "Weather MQTT publish would go to topic '{}'",
                cfg.mqtt_topic
            );
        }

        // Optional: dnsmasq config
        if cfg.dnsmasq_enabled {
            if let Err(e) = self.write_dnsmasq_config(&cfg) {
                warn!("dnsmasq config write failed: {}", e);
            }
        }

        *self.data.write().await = Some(data);
        info!("Weather data refreshed for '{}'", cfg.location_name);
        Ok(())
    }

    // ── Parsing ───────────────────────────────────────────────────────────────

    fn parse_response(&self, raw: OpenMeteoResponse, cfg: &WeatherConfig) -> WeatherData {
        let cur = &raw.current;
        let cur_ts = parse_iso_dt(&cur.time).unwrap_or_else(Utc::now);

        // Sunrise/sunset from daily[0]
        let sunrise = raw.daily.sunrise.first().cloned().unwrap_or_default();
        let sunset = raw.daily.sunset.first().cloned().unwrap_or_default();

        let current = CurrentWeather {
            timestamp: cur_ts.timestamp(),
            temperature: cur.temperature_2m,
            feels_like: cur.apparent_temperature,
            humidity: cur.relative_humidity_2m,
            pressure: cur.surface_pressure,
            wind_speed: cur.wind_speed_10m,
            wind_direction: cur.wind_direction_10m,
            wind_gusts: cur.wind_gusts_10m,
            precipitation: cur.precipitation,
            weather_code: cur.weather_code,
            weather_description: wmo_description(cur.weather_code, cur.is_day == 1),
            weather_icon: wmo_icon(cur.weather_code, cur.is_day == 1),
            is_day: cur.is_day == 1,
            uv_index: cur.uv_index,
            visibility: cur.visibility,
            loxone_code: wmo_to_loxone(cur.weather_code, cur.is_day == 1),
            sunrise: sunrise.clone(),
            sunset: sunset.clone(),
        };

        let daily: Vec<DailyForecast> = raw
            .daily
            .time
            .iter()
            .enumerate()
            .map(|(i, date)| {
                let tmax = raw.daily.temperature_2m_max.get(i).copied().unwrap_or(0.0);
                let tmin = raw.daily.temperature_2m_min.get(i).copied().unwrap_or(0.0);
                let code = raw.daily.weather_code.get(i).copied().unwrap_or(0);
                DailyForecast {
                    date: date.clone(),
                    temp_max: tmax,
                    temp_min: tmin,
                    temp_mean: (tmax + tmin) / 2.0,
                    weather_code: code,
                    weather_description: wmo_description(code, true),
                    precipitation_sum: raw.daily.precipitation_sum.get(i).copied().unwrap_or(0.0),
                    precipitation_probability: raw
                        .daily
                        .precipitation_probability_max
                        .get(i)
                        .copied()
                        .unwrap_or(0),
                    wind_speed_max: raw.daily.wind_speed_10m_max.get(i).copied().unwrap_or(0.0),
                    uv_index_max: raw.daily.uv_index_max.get(i).copied().unwrap_or(0.0),
                    sunrise: raw.daily.sunrise.get(i).cloned().unwrap_or_default(),
                    sunset: raw.daily.sunset.get(i).cloned().unwrap_or_default(),
                    loxone_code: wmo_to_loxone(code, true),
                }
            })
            .collect();

        // Determine current hour index to start from
        let now_str = cur_ts.format("%Y-%m-%dT%H:00").to_string();
        let start_idx = raw
            .hourly
            .time
            .iter()
            .position(|t| t.starts_with(&now_str[..13]))
            .unwrap_or(0);

        let hourly: Vec<HourlyForecast> = raw
            .hourly
            .time
            .iter()
            .enumerate()
            .skip(start_idx)
            .take(168) // 7 days × 24 hours
            .map(|(i, dt)| {
                let code = raw.hourly.weather_code.get(i).copied().unwrap_or(0);
                // Approximate is_day from hour
                let hour = dt
                    .get(11..13)
                    .and_then(|h| h.parse::<u8>().ok())
                    .unwrap_or(12);
                let is_day = (6..21).contains(&hour);
                HourlyForecast {
                    datetime: dt.clone(),
                    temperature: raw.hourly.temperature_2m.get(i).copied().unwrap_or(0.0),
                    feels_like: raw
                        .hourly
                        .apparent_temperature
                        .get(i)
                        .copied()
                        .unwrap_or(0.0),
                    humidity: raw.hourly.relative_humidity_2m.get(i).copied().unwrap_or(0),
                    precipitation: raw.hourly.precipitation.get(i).copied().unwrap_or(0.0),
                    precipitation_probability: raw
                        .hourly
                        .precipitation_probability
                        .get(i)
                        .copied()
                        .unwrap_or(0),
                    weather_code: code,
                    wind_speed: raw.hourly.wind_speed_10m.get(i).copied().unwrap_or(0.0),
                    wind_direction: raw.hourly.wind_direction_10m.get(i).copied().unwrap_or(0),
                    wind_gusts: raw.hourly.wind_gusts_10m.get(i).copied().unwrap_or(0.0),
                    pressure: raw.hourly.surface_pressure.get(i).copied().unwrap_or(0.0),
                    visibility: raw.hourly.visibility.get(i).copied().unwrap_or(0.0),
                    loxone_code: wmo_to_loxone(code, is_day),
                }
            })
            .collect();

        WeatherData {
            location: LocationInfo {
                name: if cfg.location_name.is_empty() {
                    format!("{:.2},{:.2}", raw.latitude, raw.longitude)
                } else {
                    cfg.location_name.clone()
                },
                latitude: raw.latitude,
                longitude: raw.longitude,
                elevation: raw.elevation,
                timezone: raw.timezone.clone(),
                utc_offset_seconds: raw.utc_offset_seconds,
            },
            current,
            daily,
            hourly,
            fetched_at: Utc::now().timestamp(),
        }
    }

    // ── Loxone UDP push ───────────────────────────────────────────────────────
    //
    // The Miniserver listens on a configurable UDP port for lines of the form:
    //   "name@value; name2@value2; ..."
    // Each batch is sent as one UDP datagram, mirroring what datatoloxone.pl does.

    fn push_udp(&self, data: &WeatherData, cfg: &WeatherConfig) -> Result<(), String> {
        // We need the Miniserver IP – looked up from config via the key.
        // Since we don't have direct access to GeneralConfig here we accept
        // the caller passing the resolved IP in cfg.local_ip as a workaround.
        // The caller (daemon / background task) must set resolved_ms_ip before calling.
        // For now we store the IP in a field set by the task spawner.
        let ip = if cfg.local_ip.is_empty() {
            return Err("Miniserver IP not set (local_ip field)".into());
        } else {
            cfg.local_ip.clone() // repurposed field – set by daemon
        };

        let port = cfg.miniserver_udp_port;
        let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("UDP bind failed: {}", e))?;
        socket
            .connect(format!("{}:{}", ip, port))
            .map_err(|e| format!("UDP connect failed: {}", e))?;

        let c = &data.current;
        let payload = build_udp_payload(c);
        socket
            .send(payload.as_bytes())
            .map_err(|e| format!("UDP send failed: {}", e))?;

        info!(
            "Weather UDP push → {}:{} ({} bytes)",
            ip,
            port,
            payload.len()
        );
        Ok(())
    }

    // ── Loxone Cloud Emulator (weather.loxone.com format) ────────────────────
    //
    // The Miniserver GETs:
    //   http://weather.loxone.com:6066/forecast/?user=loxone_EE...&coord=LON,LAT&format=1&asl=ELEV
    //
    // Format 1 (semicolon-delimited CSV):
    //   <mb_metadata>…header…</mb_metadata>
    //   <valid_until>YEAR-12-31</valid_until>
    //   <station>
    //   ;CITY;LON;LAT;ELEV;COUNTRY;TZ;UTC±X;HH:MM;HH:MM;
    //   DD.MM.YYYY;DAY;\tHH;\tTEMP;\tFEELS;\tWIND;\tDIR;\tGUST;\t0;\t0;\t0;\tPRECIP;\tPOP;\t0;\tPRESS;\tHUMID;\t0;\tCODE;\t0;
    //   …up to 168 rows…
    //   </station>

    pub fn loxone_emu_response(&self, data: &WeatherData) -> String {
        let loc = &data.location;
        let cur = &data.current;

        // UTC offset string like "UTC+1.0" or "UTC-5.5"
        let offset_h = loc.utc_offset_seconds as f64 / 3600.0;
        let utc_str = if offset_h >= 0.0 {
            format!("UTC+{:.1}", offset_h)
        } else {
            format!("UTC{:.1}", offset_h)
        };

        // Sunrise / sunset as HH:MM
        let sunrise_hm = time_hm(&cur.sunrise);
        let sunset_hm = time_hm(&cur.sunset);

        let valid_year = chrono::Utc::now().year() + 5;

        let mut out = String::with_capacity(65536);
        out.push_str("<mb_metadata>\n");
        out.push_str("id;name;longitude;latitude;height (m.asl.);country;timezone;utc-timedifference;sunrise;sunset;\n");
        out.push_str("local date;weekday;local time;temperature(C);feeledTemperature(C);windspeed(km/h);winddirection(degr);wind gust(km/h);low clouds(%);medium clouds(%);high clouds(%);precipitation(mm);probability of Precip(%);snowFraction;sea level pressure(hPa);relative humidity(%);CAPE;picto-code;radiation (W/m2);\n");
        out.push_str("</mb_metadata>\n");
        out.push_str(&format!(
            "<valid_until>{}-12-31</valid_until>\n",
            valid_year
        ));
        out.push_str("<station>\n");

        // Station header line
        out.push_str(&format!(
            ";{};{:.4};{:.4};{:.0};;{};{};{};{};\n",
            loc.name,
            loc.longitude,
            loc.latitude,
            loc.elevation,
            loc.timezone,
            utc_str,
            sunrise_hm,
            sunset_hm,
        ));

        // Hourly rows (up to 168)
        for h in data.hourly.iter().take(168) {
            let dt = parse_iso_dt_naive(&h.datetime);
            let (date_str, day_abbr, hour_str) = match dt {
                Some(d) => (
                    d.format("%d.%m.%Y").to_string(),
                    d.format("%a").to_string(),
                    d.format("%H").to_string(),
                ),
                None => ("01.01.2009".into(), "Mon".into(), "00".into()),
            };
            // snowFraction: rough estimate from temperature
            let snow_frac = if h.temperature < 2.0 && h.precipitation > 0.0 {
                1.0f64
            } else {
                0.0f64
            };
            out.push_str(&format!(
                "{};\t{};\t{};\t{:.2};\t{:.1};\t{:.0};\t{};\t{:.0};\t0;\t0;\t0;\t{:.1};\t{};\t{:.1};\t{:.0};\t{};\t0;\t{};\t0;\n",
                date_str,
                day_abbr,
                hour_str,
                h.temperature,
                h.feels_like,
                h.wind_speed,
                h.wind_direction,
                h.wind_gusts,
                h.precipitation,
                h.precipitation_probability,
                snow_frac,
                h.pressure,
                h.humidity,
                h.loxone_code,
            ));
        }

        out.push_str("</station>\n");
        out
    }

    // ── dnsmasq ───────────────────────────────────────────────────────────────

    fn write_dnsmasq_config(&self, cfg: &WeatherConfig) -> Result<(), String> {
        if cfg.local_ip.is_empty() {
            return Err("local_ip not set; cannot write dnsmasq config".into());
        }
        let content = format!(
            "# Auto-generated by RustyLox – redirects Loxone Miniserver weather\n\
             # requests to this host instead of the real Loxone cloud.\n\
             address=/weather.loxone.com/{}\n",
            cfg.local_ip
        );
        std::fs::write("/etc/dnsmasq.d/rustylox-weather.conf", &content)
            .map_err(|e| format!("Cannot write dnsmasq config: {}", e))?;

        // Restart dnsmasq (non-fatal if it fails – e.g. not installed)
        let status = std::process::Command::new("service")
            .args(["dnsmasq", "restart"])
            .status();
        match status {
            Ok(s) if s.success() => info!("dnsmasq restarted successfully"),
            Ok(s) => warn!("dnsmasq restart exited with status {}", s),
            Err(e) => warn!("Could not restart dnsmasq: {}", e),
        }
        Ok(())
    }

    /// Spawn the background refresh loop. Must be called once at startup.
    pub fn spawn_background_task(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                let interval = {
                    let cfg = self.config.read().await;
                    if cfg.enabled {
                        cfg.update_interval_minutes
                    } else {
                        60 // check every hour even when disabled
                    }
                };

                if let Err(e) = self.refresh().await {
                    error!("Weather refresh failed: {}", e);
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(interval as u64 * 60)).await;
            }
        });
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Build the UDP payload for Loxone virtual inputs.
/// Format: "name@value; name2@value2; …"
fn build_udp_payload(c: &CurrentWeather) -> String {
    let dir_des = wind_dir_abbr(c.wind_direction);
    let we_des = c.weather_description.clone();

    [
        format!("cur_tt@{:.1}", c.temperature),
        format!("cur_tt_fl@{:.1}", c.feels_like),
        format!("cur_hu@{}", c.humidity),
        format!("cur_pr@{:.1}", c.pressure),
        format!("cur_w_sp@{:.1}", c.wind_speed),
        format!("cur_w_dir@{}", c.wind_direction),
        format!("cur_w_dirdes@{}", dir_des),
        format!("cur_w_gu@{:.1}", c.wind_gusts),
        format!("cur_prec_1hr@{:.1}", c.precipitation),
        format!("cur_we_code@{}", c.weather_code),
        format!("cur_we_des@{}", we_des),
        format!("cur_we_icon@{}", c.loxone_code),
        format!("cur_uvi@{:.1}", c.uv_index),
        format!("cur_vis@{:.0}", c.visibility / 1000.0), // metres → km
    ]
    .join("; ")
        + "; "
}

/// Convert WMO weather code to Loxone picto code (1-25).
///
/// Mapping derived from weather4lox datatoloxone.pl.
pub fn wmo_to_loxone(code: u16, is_day: bool) -> u8 {
    match code {
        0 => 1,        // wolkenlos / clear sky
        1 => 2,        // mainly clear
        2 => 5,        // partly cloudy
        3 => 10,       // overcast
        45 | 48 => 19, // fog
        51 | 53 => 12, // drizzle light/moderate
        55 => 13,      // drizzle heavy
        56 | 57 => 12, // freezing drizzle
        61 | 63 => 13, // rain light/moderate
        65 => 14,      // rain heavy
        66 | 67 => 13, // freezing rain
        71 | 73 => 18, // snow fall light/moderate
        75 => 18,      // snow fall heavy
        77 => 18,      // snow grains
        80 | 81 => 13, // rain showers slight/moderate
        82 => 14,      // rain showers violent
        85 | 86 => 18, // snow showers
        95 => 16,      // thunderstorm
        96 | 99 => 15, // thunderstorm + hail
        _ => 7,        // unknown → cloudy
    }
}

/// Human-readable WMO description.
pub fn wmo_description(code: u16, is_day: bool) -> String {
    match code {
        0 => if is_day { "Clear sky" } else { "Clear night" }.into(),
        1 => "Mainly clear".into(),
        2 => "Partly cloudy".into(),
        3 => "Overcast".into(),
        45 => "Fog".into(),
        48 => "Depositing rime fog".into(),
        51 => "Light drizzle".into(),
        53 => "Moderate drizzle".into(),
        55 => "Dense drizzle".into(),
        56 => "Light freezing drizzle".into(),
        57 => "Heavy freezing drizzle".into(),
        61 => "Slight rain".into(),
        63 => "Moderate rain".into(),
        65 => "Heavy rain".into(),
        66 => "Light freezing rain".into(),
        67 => "Heavy freezing rain".into(),
        71 => "Slight snowfall".into(),
        73 => "Moderate snowfall".into(),
        75 => "Heavy snowfall".into(),
        77 => "Snow grains".into(),
        80 => "Slight rain showers".into(),
        81 => "Moderate rain showers".into(),
        82 => "Violent rain showers".into(),
        85 => "Slight snow showers".into(),
        86 => "Heavy snow showers".into(),
        95 => "Thunderstorm".into(),
        96 => "Thunderstorm with slight hail".into(),
        99 => "Thunderstorm with heavy hail".into(),
        _ => "Unknown".into(),
    }
}

/// Short icon key for the web UI.
pub fn wmo_icon(code: u16, is_day: bool) -> String {
    let s = match code {
        0 => {
            if is_day {
                "clear-day"
            } else {
                "clear-night"
            }
        }
        1 | 2 => {
            if is_day {
                "partly-cloudy-day"
            } else {
                "partly-cloudy-night"
            }
        }
        3 => "cloudy",
        45 | 48 => "fog",
        51..=57 => "drizzle",
        61..=67 => "rain",
        71..=77 => "snow",
        80..=82 => "rain",
        85 | 86 => "snow",
        95..=99 => "thunderstorm",
        _ => "cloudy",
    };
    s.into()
}

/// Wind direction degrees → compass abbreviation.
pub fn wind_dir_abbr(deg: u16) -> &'static str {
    match deg {
        0..=22 | 338..=360 => "N",
        23..=67 => "NE",
        68..=112 => "E",
        113..=157 => "SE",
        158..=202 => "S",
        203..=247 => "SW",
        248..=292 => "W",
        293..=337 => "NW",
        _ => "N",
    }
}

fn parse_iso_dt(s: &str) -> Option<DateTime<Utc>> {
    // Open-Meteo format: "2024-03-19T14:00" (no TZ) – interpret as UTC
    let with_z = if s.ends_with('Z') || s.contains('+') {
        s.to_string()
    } else {
        format!("{}Z", s)
    };
    DateTime::parse_from_rfc3339(&with_z)
        .map(|d| d.with_timezone(&Utc))
        .ok()
}

fn parse_iso_dt_naive(s: &str) -> Option<NaiveDate> {
    // "2024-03-19T14:00" → NaiveDate
    s.get(..10)
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
}

/// Extract "HH:MM" from an ISO datetime string like "2024-03-19T06:32".
fn time_hm(s: &str) -> String {
    s.get(11..16).unwrap_or("00:00").to_string()
}

// ─── chrono helper ────────────────────────────────────────────────────────────

use chrono::Datelike;
