//! Auth crate - Authentication, Authorization, and Audit for RustyLox
//!
//! Provides:
//! - JWT-based authentication
//! - Password hashing with Argon2
//! - Role-based access control (RBAC)
//! - API key management
//! - In-memory session management
//! - Audit logging

pub mod audit;
pub mod error;
pub mod jwt;
pub mod models;
pub mod password;
pub mod service;
pub mod session;
pub mod store;

pub use audit::{AuditAction, AuditLogger};
pub use error::AuthError;
pub use jwt::JwtConfig;
pub use models::{
    Action, ApiKey, AuthIdentity, Claims, IdentityKind, Resource, Role, Session, TokenResponse,
    User,
};
pub use service::AuthService;
pub use session::SessionStore;
pub use store::AuthStore;
