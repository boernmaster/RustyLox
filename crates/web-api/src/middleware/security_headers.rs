//! Security headers middleware
//!
//! Adds security-related HTTP response headers to all responses:
//! - X-Content-Type-Options: nosniff
//! - X-Frame-Options: SAMEORIGIN
//! - X-XSS-Protection: 1; mode=block
//! - Referrer-Policy: strict-origin-when-cross-origin
//! - Permissions-Policy
//! - Content-Security-Policy

use axum::{
    body::Body,
    http::{header::HeaderName, HeaderValue, Request, Response},
    middleware::Next,
};

/// Axum middleware function that injects security headers into every response
pub async fn add_security_headers(req: Request<Body>, next: Next) -> Response<Body> {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    let pairs: &[(&str, &str)] = &[
        ("x-content-type-options", "nosniff"),
        ("x-frame-options", "SAMEORIGIN"),
        ("x-xss-protection", "1; mode=block"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
        (
            "content-security-policy",
            "default-src 'self'; \
             script-src 'self' 'unsafe-inline'; \
             style-src 'self' 'unsafe-inline'; \
             img-src 'self' data:; \
             connect-src 'self'; \
             font-src 'self'; \
             object-src 'none'; \
             frame-ancestors 'self'",
        ),
    ];

    for (name, value) in pairs {
        if let (Ok(n), Ok(v)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            headers.insert(n, v);
        }
    }

    response
}
