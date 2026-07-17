//! Forwards schema/config calls to a registered addon instance's own
//! config API. An addon being unreachable must never crash or hang RustyLox -
//! every call here returns a typed error the caller turns into a clean
//! "addon offline" response.

use serde_json::Value;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("addon unreachable: {0}")]
    Unreachable(String),
    #[error("addon returned an error status: {0}")]
    BadStatus(reqwest::StatusCode),
    #[error("failed to parse addon response: {0}")]
    InvalidResponse(String),
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("failed to build addon proxy HTTP client")
}

pub async fn fetch_schema(base_url: &str) -> Result<Value, ProxyError> {
    get_json(base_url, "/addon/schema").await
}

pub async fn fetch_config(base_url: &str) -> Result<Value, ProxyError> {
    get_json(base_url, "/addon/config").await
}

async fn get_json(base_url: &str, path: &str) -> Result<Value, ProxyError> {
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let response = client()
        .get(&url)
        .send()
        .await
        .map_err(|e| ProxyError::Unreachable(e.to_string()))?;
    if !response.status().is_success() {
        return Err(ProxyError::BadStatus(response.status()));
    }
    response
        .json::<Value>()
        .await
        .map_err(|e| ProxyError::InvalidResponse(e.to_string()))
}

pub async fn save_config(base_url: &str, payload: &Value) -> Result<(), ProxyError> {
    let url = format!("{}/addon/config", base_url.trim_end_matches('/'));
    let response = client()
        .post(&url)
        .json(payload)
        .send()
        .await
        .map_err(|e| ProxyError::Unreachable(e.to_string()))?;
    if !response.status().is_success() {
        return Err(ProxyError::BadStatus(response.status()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use axum::{Json, Router};
    use serde_json::json;
    use tokio::net::TcpListener;

    async fn spawn_fake_addon() -> String {
        async fn schema() -> Json<Value> {
            Json(
                json!([{"key": "MQTT_HOST", "label": "MQTT Host", "type": "text", "help": "", "secret": false}]),
            )
        }
        async fn config() -> Json<Value> {
            Json(json!({"MQTT_HOST": {"value": "10.0.0.32", "secret_set": false}}))
        }
        async fn save(Json(_body): Json<Value>) -> Json<Value> {
            Json(json!({"saved": true}))
        }

        let app = Router::new()
            .route("/addon/schema", get(schema))
            .route("/addon/config", get(config).post(save));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{}", addr)
    }

    #[tokio::test]
    async fn fetch_schema_returns_addon_fields() {
        let base_url = spawn_fake_addon().await;

        let schema = fetch_schema(&base_url).await.expect("should succeed");

        assert_eq!(schema[0]["key"], "MQTT_HOST");
    }

    #[tokio::test]
    async fn fetch_config_returns_current_values() {
        let base_url = spawn_fake_addon().await;

        let config = fetch_config(&base_url).await.expect("should succeed");

        assert_eq!(config["MQTT_HOST"]["value"], "10.0.0.32");
    }

    #[tokio::test]
    async fn save_config_posts_and_succeeds() {
        let base_url = spawn_fake_addon().await;

        let result = save_config(&base_url, &json!({"MQTT_HOST": "10.0.0.99"})).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn fetch_schema_returns_unreachable_for_dead_address() {
        let result = fetch_schema("http://127.0.0.1:1").await;

        assert!(matches!(result, Err(ProxyError::Unreachable(_))));
    }
}
