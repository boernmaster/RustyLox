//! Auth service - orchestrates login, token validation, and API key auth

use std::sync::Arc;

use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::Rng;
use sha2::{Digest, Sha256};
use tracing::info;
use uuid::Uuid;

use crate::audit::{AuditAction, AuditLogger};
use crate::error::AuthError;
use crate::jwt::{generate_token, validate_token, JwtConfig};
use crate::models::{
    Action, ApiKey, AuthIdentity, IdentityKind, Resource, Role, TokenResponse, User,
};
use crate::password::{hash_password, verify_password};
use crate::session::SessionStore;
use crate::store::AuthStore;

/// Maximum consecutive failed attempts before lockout
const MAX_FAILED_ATTEMPTS: u32 = 5;
/// Lockout duration in minutes
const LOCKOUT_MINUTES: i64 = 15;

/// Central authentication service
#[derive(Clone)]
pub struct AuthService {
    pub store: Arc<AuthStore>,
    pub sessions: Arc<SessionStore>,
    pub jwt_config: Arc<JwtConfig>,
    pub audit: Arc<AuditLogger>,
}

impl AuthService {
    pub fn new(store: AuthStore, audit: AuditLogger) -> Self {
        Self {
            store: Arc::new(store),
            sessions: Arc::new(SessionStore::new()),
            jwt_config: Arc::new(JwtConfig::default()),
            audit: Arc::new(audit),
        }
    }

    /// Initialise defaults (creates admin user if none exist)
    pub async fn init(&self) -> Result<(), AuthError> {
        self.store.init_defaults().await
    }

    /// Authenticate a user with username + password; returns JWT on success
    pub async fn login(
        &self,
        username: &str,
        password: &str,
        ip_address: &str,
    ) -> Result<TokenResponse, AuthError> {
        let mut user = self
            .store
            .find_user_by_username(username)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        if !user.enabled {
            self.audit
                .log(
                    username,
                    AuditAction::LoginFailed,
                    "auth",
                    ip_address,
                    false,
                    Some("account disabled".into()),
                )
                .await;
            return Err(AuthError::AccountDisabled);
        }

        if user.is_locked() {
            self.audit
                .log(
                    username,
                    AuditAction::LoginFailed,
                    "auth",
                    ip_address,
                    false,
                    Some("account locked".into()),
                )
                .await;
            return Err(AuthError::AccountLocked);
        }

        if !verify_password(password, &user.password_hash)? {
            user.failed_attempts += 1;
            if user.failed_attempts >= MAX_FAILED_ATTEMPTS {
                user.locked_until = Some(Utc::now() + chrono::Duration::minutes(LOCKOUT_MINUTES));
                self.audit
                    .log(
                        username,
                        AuditAction::AccountLocked,
                        "auth",
                        ip_address,
                        false,
                        None,
                    )
                    .await;
            }
            self.store.update_user(user).await?;
            self.audit
                .log(
                    username,
                    AuditAction::LoginFailed,
                    "auth",
                    ip_address,
                    false,
                    None,
                )
                .await;
            return Err(AuthError::InvalidCredentials);
        }

        // Successful login
        user.failed_attempts = 0;
        user.locked_until = None;
        user.last_login = Some(Utc::now());
        self.store.update_user(user.clone()).await?;

        let token = generate_token(&self.jwt_config, &user.id, &user.username, &user.roles)?;

        self.audit
            .log(username, AuditAction::Login, "auth", ip_address, true, None)
            .await;

        info!("User '{}' logged in from {}", username, ip_address);

        Ok(TokenResponse {
            access_token: token,
            token_type: "Bearer".into(),
            expires_in: self.jwt_config.expiry_seconds,
            username: user.username,
            roles: user.roles,
        })
    }

    /// Validate a JWT Bearer token; returns AuthIdentity on success
    pub async fn authenticate_token(&self, token: &str) -> Result<AuthIdentity, AuthError> {
        let claims = validate_token(&self.jwt_config, token)?;
        let user_id: Uuid = claims
            .sub
            .parse()
            .map_err(|_| AuthError::InvalidToken("invalid sub".into()))?;

        Ok(AuthIdentity {
            user_id,
            username: claims.username,
            roles: claims.roles,
            kind: IdentityKind::Session(claims.jti),
        })
    }

    /// Validate an API key (prefix `lbx_`); returns AuthIdentity on success
    pub async fn authenticate_api_key(&self, raw_key: &str) -> Result<AuthIdentity, AuthError> {
        let key_hash = hash_key(raw_key);
        let api_key = self
            .store
            .find_api_key_by_hash(&key_hash)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        if api_key.is_expired() {
            return Err(AuthError::TokenExpired);
        }

        let user = self
            .store
            .find_user_by_id(&api_key.user_id)
            .await?
            .ok_or(AuthError::UserNotFound(api_key.user_id.to_string()))?;

        if !user.enabled {
            return Err(AuthError::AccountDisabled);
        }

        // Update last_used timestamp asynchronously
        let store = Arc::clone(&self.store);
        let key_id = api_key.id;
        tokio::spawn(async move {
            let _ = store.update_api_key_last_used(&key_id).await;
        });

        Ok(AuthIdentity {
            user_id: user.id,
            username: user.username,
            roles: user.roles,
            kind: IdentityKind::ApiKey(api_key.id),
        })
    }

    // --- User management ---

    pub async fn create_user(
        &self,
        actor: &AuthIdentity,
        username: impl Into<String>,
        password: impl Into<String>,
        email: impl Into<String>,
        roles: Vec<Role>,
        ip_address: &str,
    ) -> Result<User, AuthError> {
        if !actor.can(&Resource::Users, &Action::Write) {
            return Err(AuthError::Forbidden);
        }
        let username = username.into();
        let hash = hash_password(&password.into())?;
        let user = User::new(username.clone(), hash, email, roles);
        let user = self.store.create_user(user).await?;
        self.audit
            .log(
                &actor.username,
                AuditAction::CreateUser,
                &username,
                ip_address,
                true,
                None,
            )
            .await;
        Ok(user)
    }

    pub async fn change_password(
        &self,
        actor: &AuthIdentity,
        target_user_id: &Uuid,
        new_password: &str,
        ip_address: &str,
    ) -> Result<(), AuthError> {
        // Users can change their own password; admins can change any
        if actor.user_id != *target_user_id && !actor.is_admin() {
            return Err(AuthError::Forbidden);
        }
        let mut user = self
            .store
            .find_user_by_id(target_user_id)
            .await?
            .ok_or_else(|| AuthError::UserNotFound(target_user_id.to_string()))?;
        user.password_hash = hash_password(new_password)?;
        self.store.update_user(user.clone()).await?;
        self.audit
            .log(
                &actor.username,
                AuditAction::PasswordChanged,
                &user.username,
                ip_address,
                true,
                None,
            )
            .await;
        Ok(())
    }

    pub async fn delete_user(
        &self,
        actor: &AuthIdentity,
        target_user_id: &Uuid,
        ip_address: &str,
    ) -> Result<(), AuthError> {
        if !actor.is_admin() {
            return Err(AuthError::Forbidden);
        }
        let user = self
            .store
            .find_user_by_id(target_user_id)
            .await?
            .ok_or_else(|| AuthError::UserNotFound(target_user_id.to_string()))?;
        self.store.delete_user(target_user_id).await?;
        self.audit
            .log(
                &actor.username,
                AuditAction::DeleteUser,
                &user.username,
                ip_address,
                true,
                None,
            )
            .await;
        Ok(())
    }

    // --- API key management ---

    /// Create a new API key; returns the raw key (only time it is visible)
    pub async fn create_api_key(
        &self,
        actor: &AuthIdentity,
        name: impl Into<String>,
        permissions: Vec<(Resource, Action)>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        ip_address: &str,
    ) -> Result<(ApiKey, String), AuthError> {
        let raw_key = generate_raw_key();
        let key_hash = hash_key(&raw_key);
        let api_key = ApiKey {
            id: Uuid::new_v4(),
            key_hash,
            name: name.into(),
            user_id: actor.user_id,
            permissions,
            expires_at,
            last_used: None,
            created_at: Utc::now(),
        };
        let api_key = self.store.create_api_key(api_key).await?;
        self.audit
            .log(
                &actor.username,
                AuditAction::CreateApiKey,
                &api_key.name,
                ip_address,
                true,
                None,
            )
            .await;
        Ok((api_key, raw_key))
    }

    pub async fn delete_api_key(
        &self,
        actor: &AuthIdentity,
        key_id: &Uuid,
        ip_address: &str,
    ) -> Result<(), AuthError> {
        self.store.delete_api_key(key_id, &actor.user_id).await?;
        self.audit
            .log(
                &actor.username,
                AuditAction::DeleteApiKey,
                &key_id.to_string(),
                ip_address,
                true,
                None,
            )
            .await;
        Ok(())
    }
}

/// Generate a random API key with `lbx_` prefix
fn generate_raw_key() -> String {
    let suffix: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(40)
        .map(char::from)
        .collect();
    format!("lbx_{}", suffix)
}

/// SHA-256 hash of a raw API key
pub fn hash_key(raw_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    format!("{:x}", hasher.finalize())
}
