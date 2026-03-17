//! JWT token generation and validation

use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

use crate::error::AuthError;
use crate::models::{Claims, Role};

/// JWT configuration
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    /// Token lifetime in seconds (default 3600 = 1 hour)
    pub expiry_seconds: i64,
}

impl JwtConfig {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            expiry_seconds: 3600,
        }
    }

    pub fn with_expiry(mut self, seconds: i64) -> Self {
        self.expiry_seconds = seconds;
        self
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        // In production this should come from a secure env var
        let secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "change-me-in-production-loxberry-secret-key-32bytes".to_string());
        Self::new(secret)
    }
}

/// Generate a signed JWT for the given user
pub fn generate_token(
    config: &JwtConfig,
    user_id: &Uuid,
    username: &str,
    roles: &[Role],
) -> Result<String, AuthError> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        roles: roles.to_vec(),
        jti: Uuid::new_v4().to_string(),
        exp: now + config.expiry_seconds,
        iat: now,
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )
    .map_err(|e| AuthError::Internal(format!("Token generation failed: {}", e)))
}

/// Validate a JWT and return its claims
pub fn validate_token(config: &JwtConfig, token: &str) -> Result<Claims, AuthError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.secret.as_bytes()),
        &validation,
    )
    .map(|td| td.claims)
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
        _ => AuthError::InvalidToken(e.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate() {
        let config = JwtConfig::new("test-secret-key-for-unit-tests-only");
        let user_id = Uuid::new_v4();
        let token = generate_token(&config, &user_id, "admin", &[Role::Admin]).unwrap();
        let claims = validate_token(&config, &token).unwrap();
        assert_eq!(claims.username, "admin");
        assert_eq!(claims.roles, vec![Role::Admin]);
    }

    #[test]
    fn test_expired_token() {
        // Use a large negative expiry so the token is clearly expired
        let config = JwtConfig::new("test-secret").with_expiry(-3600);
        let user_id = Uuid::new_v4();
        let token = generate_token(&config, &user_id, "admin", &[]).unwrap();
        assert!(matches!(
            validate_token(&config, &token),
            Err(AuthError::TokenExpired)
        ));
    }
}
