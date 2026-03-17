//! Auth error types

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Account locked")]
    AccountLocked,

    #[error("Account disabled")]
    AccountDisabled,

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Insufficient permissions")]
    Forbidden,

    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("User already exists: {0}")]
    UserAlreadyExists(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal auth error: {0}")]
    Internal(String),
}

impl AuthError {
    pub fn http_status(&self) -> u16 {
        match self {
            AuthError::InvalidCredentials
            | AuthError::TokenExpired
            | AuthError::InvalidToken(_) => 401,
            AuthError::Forbidden => 403,
            AuthError::AccountLocked | AuthError::AccountDisabled => 403,
            AuthError::UserNotFound(_) | AuthError::NotFound(_) => 404,
            AuthError::UserAlreadyExists(_) => 409,
            AuthError::Internal(_) => 500,
        }
    }
}
