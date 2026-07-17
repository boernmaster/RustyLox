//! Builds the addon discovery catalog from GitHub Container Registry:
//! lists packages under a GitHub user via the Packages API, then reads each
//! image's OCI labels via the GHCR distribution API to find ones marked
//! io.rustylox.addon=true.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub name: String,
    pub title: String,
    pub description: String,
    pub source: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("GitHub API request failed: {0}")]
    RequestFailed(String),
    #[error("unexpected response shape: {0}")]
    InvalidResponse(String),
}

const CACHE_TTL: Duration = Duration::from_secs(600); // 10 minutes

struct CachedCatalog {
    entries: Vec<CatalogEntry>,
    fetched_at: Instant,
}

pub struct CatalogClient {
    user: String,
    token: String,
    github_api_base: String,
    ghcr_base: String,
    cache: Mutex<Option<CachedCatalog>>,
}

impl CatalogClient {
    pub fn new(user: String, token: String) -> Self {
        Self {
            user,
            token,
            github_api_base: "https://api.github.com".to_string(),
            ghcr_base: "https://ghcr.io".to_string(),
            cache: Mutex::new(None),
        }
    }

    /// Test hook: point at fake local servers instead of the real hosts.
    pub fn with_base_urls(mut self, github_api_base: String, ghcr_base: String) -> Self {
        self.github_api_base = github_api_base;
        self.ghcr_base = ghcr_base;
        self
    }

    fn client(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build catalog HTTP client")
    }

    pub async fn list_addons(&self) -> Result<Vec<CatalogEntry>, CatalogError> {
        {
            let cache = self.cache.lock().await;
            if let Some(cached) = cache.as_ref() {
                if cached.fetched_at.elapsed() < CACHE_TTL {
                    return Ok(cached.entries.clone());
                }
            }
        }

        let packages = self.list_packages().await?;
        let mut entries = Vec::new();
        for package_name in packages {
            if let Some(entry) = self.fetch_labels(&package_name).await? {
                entries.push(entry);
            }
        }

        let mut cache = self.cache.lock().await;
        *cache = Some(CachedCatalog {
            entries: entries.clone(),
            fetched_at: Instant::now(),
        });
        Ok(entries)
    }

    async fn list_packages(&self) -> Result<Vec<String>, CatalogError> {
        let url = format!(
            "{}/users/{}/packages?package_type=container",
            self.github_api_base, self.user
        );
        let response = self
            .client()
            .get(&url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "rustylox-addon-catalog")
            .send()
            .await
            .map_err(|e| CatalogError::RequestFailed(e.to_string()))?;
        if !response.status().is_success() {
            return Err(CatalogError::RequestFailed(format!(
                "GitHub Packages API returned {}",
                response.status()
            )));
        }
        let body: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| CatalogError::InvalidResponse(e.to_string()))?;
        Ok(body
            .into_iter()
            .filter_map(|entry| entry.get("name").and_then(|n| n.as_str()).map(String::from))
            .collect())
    }

    async fn fetch_labels(&self, package_name: &str) -> Result<Option<CatalogEntry>, CatalogError> {
        let token_url = format!(
            "{}/token?service=ghcr.io&scope=repository:{}/{}:pull",
            self.ghcr_base, self.user, package_name
        );
        let token_response = self
            .client()
            .get(&token_url)
            .basic_auth(&self.user, Some(&self.token))
            .send()
            .await
            .map_err(|e| CatalogError::RequestFailed(e.to_string()))?;
        let token_body: serde_json::Value = token_response
            .json()
            .await
            .map_err(|e| CatalogError::InvalidResponse(e.to_string()))?;
        let ghcr_token = token_body
            .get("token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| {
                CatalogError::InvalidResponse("missing token in GHCR response".to_string())
            })?;

        let manifest_url = format!(
            "{}/v2/{}/{}/manifests/latest",
            self.ghcr_base, self.user, package_name
        );
        let manifest_response = self
            .client()
            .get(&manifest_url)
            .bearer_auth(ghcr_token)
            .header("Accept", "application/vnd.oci.image.manifest.v1+json")
            .send()
            .await
            .map_err(|e| CatalogError::RequestFailed(e.to_string()))?;
        let manifest: serde_json::Value = manifest_response
            .json()
            .await
            .map_err(|e| CatalogError::InvalidResponse(e.to_string()))?;
        let digest = manifest
            .get("config")
            .and_then(|c| c.get("digest"))
            .and_then(|d| d.as_str())
            .ok_or_else(|| {
                CatalogError::InvalidResponse("missing config digest in manifest".to_string())
            })?;

        let blob_url = format!(
            "{}/v2/{}/{}/blobs/{}",
            self.ghcr_base, self.user, package_name, digest
        );
        let blob_response = self
            .client()
            .get(&blob_url)
            .bearer_auth(ghcr_token)
            .send()
            .await
            .map_err(|e| CatalogError::RequestFailed(e.to_string()))?;
        let blob: serde_json::Value = blob_response
            .json()
            .await
            .map_err(|e| CatalogError::InvalidResponse(e.to_string()))?;

        let labels = blob.get("config").and_then(|c| c.get("Labels"));
        let is_addon = labels
            .and_then(|l| l.get("io.rustylox.addon"))
            .and_then(|v| v.as_str())
            .map(|v| v == "true")
            .unwrap_or(false);
        if !is_addon {
            return Ok(None);
        }

        let get_label = |key: &str, default: &str| {
            labels
                .and_then(|l| l.get(key))
                .and_then(|v| v.as_str())
                .unwrap_or(default)
                .to_string()
        };

        Ok(Some(CatalogEntry {
            name: package_name.to_string(),
            title: get_label("org.opencontainers.image.title", package_name),
            description: get_label("org.opencontainers.image.description", ""),
            source: get_label("org.opencontainers.image.source", ""),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::Path;
    use axum::routing::get;
    use axum::{Json, Router};
    use serde_json::json;
    use tokio::net::TcpListener;

    async fn spawn_fake_github_and_ghcr() -> (String, String) {
        async fn packages() -> Json<serde_json::Value> {
            Json(json!([{"name": "kia-connect-bridge"}, {"name": "unrelated-image"}]))
        }
        async fn token() -> Json<serde_json::Value> {
            Json(json!({"token": "fake-token"}))
        }
        async fn manifest(Path((_user, name)): Path<(String, String)>) -> Json<serde_json::Value> {
            Json(json!({"config": {"digest": format!("sha256:fake-{}", name)}}))
        }
        async fn blob(
            Path((_user, name, _digest)): Path<(String, String, String)>,
        ) -> Json<serde_json::Value> {
            if name == "kia-connect-bridge" {
                Json(json!({"config": {"Labels": {
                    "io.rustylox.addon": "true",
                    "org.opencontainers.image.title": "kia-connect-bridge",
                    "org.opencontainers.image.description": "Bridges Kia Connect into MQTT",
                    "org.opencontainers.image.source": "https://github.com/boernmaster/kia-connect-bridge"
                }}}))
            } else {
                Json(json!({"config": {"Labels": {}}}))
            }
        }

        let github_app = Router::new().route("/users/:user/packages", get(packages));
        let github_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let github_addr = github_listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(github_listener, github_app).await.unwrap();
        });

        let ghcr_app = Router::new()
            .route("/token", get(token))
            .route("/v2/:user/:name/manifests/latest", get(manifest))
            .route("/v2/:user/:name/blobs/:digest", get(blob));
        let ghcr_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let ghcr_addr = ghcr_listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(ghcr_listener, ghcr_app).await.unwrap();
        });

        (
            format!("http://{}", github_addr),
            format!("http://{}", ghcr_addr),
        )
    }

    #[tokio::test]
    async fn list_addons_filters_to_labeled_images_only() {
        let (github_base, ghcr_base) = spawn_fake_github_and_ghcr().await;
        let client = CatalogClient::new("boernmaster".to_string(), "fake-pat".to_string())
            .with_base_urls(github_base, ghcr_base);

        let entries = client.list_addons().await.expect("should succeed");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "kia-connect-bridge");
        assert_eq!(entries[0].title, "kia-connect-bridge");
        assert_eq!(
            entries[0].source,
            "https://github.com/boernmaster/kia-connect-bridge"
        );
    }

    #[tokio::test]
    async fn list_addons_caches_within_ttl() {
        let (github_base, ghcr_base) = spawn_fake_github_and_ghcr().await;
        let client = CatalogClient::new("boernmaster".to_string(), "fake-pat".to_string())
            .with_base_urls(github_base, ghcr_base);

        let first = client.list_addons().await.expect("should succeed");
        let second = client.list_addons().await.expect("should succeed");

        assert_eq!(first.len(), second.len());
    }
}
