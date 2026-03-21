//! Auth data models: User, Role, Permission, ApiKey, Session

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User role
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Admin,
    Operator,
    Viewer,
    PluginManager,
}

impl Role {
    /// Check if this role can perform an action on a resource
    pub fn can(&self, resource: &Resource, action: &Action) -> bool {
        match self {
            Role::Admin => true,
            Role::Operator => match action {
                Action::Delete => matches!(resource, Resource::Logs),
                _ => !matches!(resource, Resource::Users),
            },
            Role::Viewer => matches!(action, Action::Read),
            Role::PluginManager => matches!(resource, Resource::Plugins),
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Role::Admin => "admin",
            Role::Operator => "operator",
            Role::Viewer => "viewer",
            Role::PluginManager => "plugin_manager",
        };
        write!(f, "{}", s)
    }
}

/// Protected resource
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resource {
    Miniserver,
    MqttGateway,
    Plugins,
    Settings,
    Logs,
    Backup,
    Users,
    ApiKeys,
    System,
}

/// Action on a resource
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Read,
    Write,
    Delete,
    Execute,
}

/// Stored user record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub email: String,
    pub roles: Vec<Role>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    /// Failed consecutive login attempts
    pub failed_attempts: u32,
    /// Locked until this time (if Some)
    pub locked_until: Option<DateTime<Utc>>,
}

impl User {
    pub fn new(
        username: impl Into<String>,
        password_hash: impl Into<String>,
        email: impl Into<String>,
        roles: Vec<Role>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            username: username.into(),
            password_hash: password_hash.into(),
            email: email.into(),
            roles,
            enabled: true,
            created_at: Utc::now(),
            last_login: None,
            failed_attempts: 0,
            locked_until: None,
        }
    }

    pub fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            Utc::now() < locked_until
        } else {
            false
        }
    }

    pub fn can(&self, resource: &Resource, action: &Action) -> bool {
        self.roles.iter().any(|r| r.can(resource, action))
    }
}

/// API key record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    /// SHA-256 hash of the raw key
    pub key_hash: String,
    /// Human-readable name
    pub name: String,
    pub user_id: Uuid,
    pub permissions: Vec<(Resource, Action)>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl ApiKey {
    pub fn is_expired(&self) -> bool {
        if let Some(exp) = self.expires_at {
            Utc::now() > exp
        } else {
            false
        }
    }

    pub fn can(&self, resource: &Resource, action: &Action) -> bool {
        self.permissions
            .iter()
            .any(|(r, a)| r == resource && a == action)
    }
}

/// Active session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: Uuid,
    pub username: String,
    pub roles: Vec<Role>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub ip_address: String,
}

impl Session {
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Claims embedded in a JWT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Username
    pub username: String,
    /// Roles
    pub roles: Vec<Role>,
    /// Session ID
    pub jti: String,
    /// Expiry (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
}

/// Response returned after successful login
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub username: String,
    pub roles: Vec<Role>,
}

/// Authenticated identity (extracted from JWT or API key)
#[derive(Debug, Clone)]
pub struct AuthIdentity {
    pub user_id: Uuid,
    pub username: String,
    pub roles: Vec<Role>,
    pub kind: IdentityKind,
}

#[derive(Debug, Clone)]
pub enum IdentityKind {
    Session(String),
    /// (key_id, key-specific permissions)
    ApiKey(Uuid, Vec<(Resource, Action)>),
}

impl AuthIdentity {
    pub fn can(&self, resource: &Resource, action: &Action) -> bool {
        // API key identities are constrained to their declared permissions
        // (still bounded by the user's roles)
        if let IdentityKind::ApiKey(_, permissions) = &self.kind {
            return permissions
                .iter()
                .any(|(r, a)| r == resource && a == action)
                && self.roles.iter().any(|r| r.can(resource, action));
        }
        self.roles.iter().any(|r| r.can(resource, action))
    }

    pub fn is_admin(&self) -> bool {
        self.roles.contains(&Role::Admin)
    }
}
