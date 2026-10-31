//! Guards: how a request's identity is established.
//!
//! Each guard extracts an `auth_id` from the request; the provider's resolver then
//! loads the user. Session (cookie) suits SSR; `Bearer` (JWT) suits APIs.

use axum::http::request::Parts;
use rustali_core::{AppContext, Result};

use crate::session_auth;

/// Session key holding the logged-in user's id.
pub const SESSION_USER_KEY: &str = "_rustali.auth.id";

/// One authentication mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Guard {
    /// Cookie session (`login`/`logout` write the id into the session).
    Session,
    /// `Authorization: Bearer <jwt>` (requires the `jwt` feature).
    Bearer,
}

impl Guard {
    /// Name for logs.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::Bearer => "bearer",
        }
    }

    /// Pull the `auth_id` this guard can see, if any.
    pub(crate) async fn extract_id(
        self,
        ctx: &AppContext,
        parts: &Parts,
    ) -> Result<Option<String>> {
        match self {
            Self::Session => Ok(session_auth::current_id(parts).await),
            Self::Bearer => Ok(bearer_id(ctx, parts)),
        }
    }
}

/// Extract and verify a JWT from the `Authorization` header.
#[cfg(feature = "jwt")]
fn bearer_id(ctx: &AppContext, parts: &Parts) -> Option<String> {
    let token = parts
        .headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")?;
    match crate::jwt::verify(ctx, token) {
        Ok(claims) => Some(claims.sub),
        Err(err) => {
            tracing::debug!(%err, "rejecting bearer token");
            None
        }
    }
}

#[cfg(not(feature = "jwt"))]
fn bearer_id(_ctx: &AppContext, _parts: &Parts) -> Option<String> {
    None
}
