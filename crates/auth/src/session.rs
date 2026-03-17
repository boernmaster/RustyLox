//! In-memory session management

use chrono::{Duration, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::{Role, Session};

/// In-memory session store
#[derive(Debug, Clone, Default)]
pub struct SessionStore {
    sessions: Arc<DashMap<String, Session>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new session for a user
    pub fn create(
        &self,
        user_id: Uuid,
        username: impl Into<String>,
        roles: Vec<Role>,
        ip_address: impl Into<String>,
        ttl_seconds: i64,
    ) -> Session {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let session = Session {
            id: id.clone(),
            user_id,
            username: username.into(),
            roles,
            created_at: now,
            expires_at: now + Duration::seconds(ttl_seconds),
            ip_address: ip_address.into(),
        };
        self.sessions.insert(id, session.clone());
        session
    }

    /// Get session by ID (returns None if expired)
    pub fn get(&self, id: &str) -> Option<Session> {
        let session = self.sessions.get(id)?.clone();
        if session.is_expired() {
            drop(session);
            self.sessions.remove(id);
            return None;
        }
        Some(session)
    }

    /// Remove session (logout)
    pub fn remove(&self, id: &str) {
        self.sessions.remove(id);
    }

    /// Purge all expired sessions
    pub fn purge_expired(&self) {
        self.sessions.retain(|_, s| !s.is_expired());
    }
}
