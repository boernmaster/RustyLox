//! Persistent user and API key storage (JSON-backed)

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::AuthError;
use crate::models::{ApiKey, Role, User};
use crate::password::hash_password;

#[derive(Debug, Default, Serialize, Deserialize)]
struct AuthDatabase {
    users: Vec<User>,
    api_keys: Vec<ApiKey>,
}

/// File-backed store for users and API keys
#[derive(Debug, Clone)]
pub struct AuthStore {
    path: PathBuf,
}

impl AuthStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join("auth.json"),
        }
    }

    async fn load(&self) -> Result<AuthDatabase, AuthError> {
        match fs::read_to_string(&self.path).await {
            Ok(content) => serde_json::from_str(&content)
                .map_err(|e| AuthError::Internal(format!("Failed to parse auth database: {}", e))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(AuthDatabase::default()),
            Err(e) => Err(AuthError::Internal(format!(
                "Failed to read auth database: {}",
                e
            ))),
        }
    }

    async fn save(&self, db: &AuthDatabase) -> Result<(), AuthError> {
        let content = serde_json::to_string_pretty(db).map_err(|e| {
            AuthError::Internal(format!("Failed to serialize auth database: {}", e))
        })?;

        // Write to temp file then rename for atomicity
        let tmp = self.path.with_extension("tmp");
        fs::write(&tmp, &content)
            .await
            .map_err(|e| AuthError::Internal(format!("Failed to write auth database: {}", e)))?;
        fs::rename(&tmp, &self.path)
            .await
            .map_err(|e| AuthError::Internal(format!("Failed to rename auth database: {}", e)))?;

        Ok(())
    }

    /// Initialize with a default admin user if no users exist
    pub async fn init_defaults(&self) -> Result<(), AuthError> {
        let db = self.load().await?;
        if db.users.is_empty() {
            let default_password =
                std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string());
            warn!("No users found - creating default admin user. Change the password immediately!");
            let hash = hash_password(&default_password)?;
            let admin = User::new("admin", hash, "admin@localhost", vec![Role::Admin]);
            let mut db = db;
            db.users.push(admin);
            self.save(&db).await?;
            info!("Created default admin user");
        }
        Ok(())
    }

    // --- User operations ---

    pub async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, AuthError> {
        let db = self.load().await?;
        Ok(db.users.into_iter().find(|u| u.username == username))
    }

    pub async fn find_user_by_id(&self, id: &Uuid) -> Result<Option<User>, AuthError> {
        let db = self.load().await?;
        Ok(db.users.into_iter().find(|u| u.id == *id))
    }

    pub async fn list_users(&self) -> Result<Vec<User>, AuthError> {
        let db = self.load().await?;
        Ok(db.users)
    }

    pub async fn create_user(&self, user: User) -> Result<User, AuthError> {
        let mut db = self.load().await?;
        if db.users.iter().any(|u| u.username == user.username) {
            return Err(AuthError::UserAlreadyExists(user.username.clone()));
        }
        db.users.push(user.clone());
        self.save(&db).await?;
        Ok(user)
    }

    pub async fn update_user(&self, user: User) -> Result<User, AuthError> {
        let mut db = self.load().await?;
        let pos = db
            .users
            .iter()
            .position(|u| u.id == user.id)
            .ok_or_else(|| AuthError::UserNotFound(user.id.to_string()))?;
        db.users[pos] = user.clone();
        self.save(&db).await?;
        Ok(user)
    }

    pub async fn delete_user(&self, id: &Uuid) -> Result<(), AuthError> {
        let mut db = self.load().await?;
        let before = db.users.len();
        db.users.retain(|u| u.id != *id);
        if db.users.len() == before {
            return Err(AuthError::UserNotFound(id.to_string()));
        }
        // Also remove associated API keys
        db.api_keys.retain(|k| k.user_id != *id);
        self.save(&db).await?;
        Ok(())
    }

    // --- API key operations ---

    pub async fn find_api_key_by_hash(&self, key_hash: &str) -> Result<Option<ApiKey>, AuthError> {
        let db = self.load().await?;
        Ok(db.api_keys.into_iter().find(|k| k.key_hash == key_hash))
    }

    pub async fn list_api_keys_for_user(&self, user_id: &Uuid) -> Result<Vec<ApiKey>, AuthError> {
        let db = self.load().await?;
        Ok(db
            .api_keys
            .into_iter()
            .filter(|k| k.user_id == *user_id)
            .collect())
    }

    pub async fn create_api_key(&self, key: ApiKey) -> Result<ApiKey, AuthError> {
        let mut db = self.load().await?;
        db.api_keys.push(key.clone());
        self.save(&db).await?;
        Ok(key)
    }

    pub async fn update_api_key_last_used(&self, id: &Uuid) -> Result<(), AuthError> {
        let mut db = self.load().await?;
        if let Some(key) = db.api_keys.iter_mut().find(|k| k.id == *id) {
            key.last_used = Some(chrono::Utc::now());
        }
        self.save(&db).await?;
        Ok(())
    }

    pub async fn delete_api_key(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AuthError> {
        let mut db = self.load().await?;
        let before = db.api_keys.len();
        db.api_keys
            .retain(|k| !(k.id == *id && k.user_id == *user_id));
        if db.api_keys.len() == before {
            return Err(AuthError::NotFound("API key".into()));
        }
        self.save(&db).await?;
        Ok(())
    }
}
