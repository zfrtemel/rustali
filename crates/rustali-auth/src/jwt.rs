//! JWT access tokens (HS256), signed with the application key.

use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rustali_core::{AppContext, Error, Result};
use serde::{Deserialize, Serialize};

/// Registered claims Rustali issues and checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject: the user's `auth_id`.
    pub sub: String,
    /// Expiry, seconds since the Unix epoch.
    pub exp: i64,
    /// Issued-at, seconds since the Unix epoch.
    pub iat: i64,
}

/// Mint an access token for `sub`, valid for `ttl_secs`.
pub fn issue(ctx: &AppContext, sub: &str, ttl_secs: i64) -> Result<String> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: sub.to_owned(),
        iat: now,
        exp: now + ttl_secs,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(&key(ctx)?),
    )
    .map_err(Error::internal)
}

/// Verify a token's signature and expiry, returning its claims.
pub fn verify(ctx: &AppContext, token: &str) -> Result<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = 5; // tolerate small clock skew
    decode::<Claims>(token, &DecodingKey::from_secret(&key(ctx)?), &validation)
        .map(|data| data.claims)
        .map_err(|_| Error::Unauthorized)
}

/// The signing key from `app.key`. Tokens require a configured key even in dev.
fn key(ctx: &AppContext) -> Result<Vec<u8>> {
    let key = &ctx.config().app.key;
    if key.len() < 16 {
        return Err(Error::internal(JwtKeyMissing));
    }
    Ok(key.as_bytes().to_vec())
}

/// Raised when JWTs are used without a sufficiently long `app.key`.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("app.key must be at least 16 characters to issue or verify JWTs")]
pub struct JwtKeyMissing;
