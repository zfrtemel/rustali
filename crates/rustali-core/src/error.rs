//! The framework-wide error type and its HTTP representation.
//!
//! Every fallible framework API returns [`Result<T>`]. Handlers return it too, and the
//! [`IntoResponse`] implementation turns an [`Error`] into either an RFC 9457 *Problem
//! Details* JSON body (when the client prefers `application/json`) or an HTML error
//! page. This is the "exception filter" of the pipeline; applications can override
//! the rendering through the app builder's error handler hook.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Convenience alias used throughout the framework.
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Framework-wide error type.
///
/// Variants map to HTTP semantics so that handlers can simply `?` and get a correct
/// status code. Infrastructure failures are wrapped in [`Error::Internal`] and never
/// leak details to clients outside debug mode.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The requested resource does not exist (404).
    #[error("not found")]
    NotFound,

    /// The request is syntactically invalid (400).
    #[error("bad request: {0}")]
    BadRequest(String),

    /// Authentication is required or failed (401).
    #[error("unauthorized")]
    Unauthorized,

    /// The caller is authenticated but not allowed (403).
    #[error("forbidden")]
    Forbidden,

    /// Configuration could not be loaded or is invalid (startup only).
    ///
    /// Boxed because `figment::Error` is large and would bloat every `Result<T>`.
    #[error("configuration error: {0}")]
    Config(Box<figment::Error>),

    /// Any other failure; rendered as 500.
    #[error(transparent)]
    Internal(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl Error {
    /// Wrap an arbitrary error as an internal server error.
    pub fn internal<E>(err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Internal(Box::new(err))
    }

    /// The HTTP status this error renders as.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Config(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::internal(err)
    }
}

impl From<figment::Error> for Error {
    fn from(err: figment::Error) -> Self {
        Self::Config(Box::new(err))
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        // Content negotiation (JSON vs HTML) lands in Faz 1 together with the
        // `Accept`-aware renderer; until then every error is a plain status.
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(error = %self, "request failed");
        }
        (status, status.canonical_reason().unwrap_or("error")).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_codes_follow_http_semantics() {
        assert_eq!(Error::NotFound.status(), StatusCode::NOT_FOUND);
        assert_eq!(Error::Unauthorized.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(Error::Forbidden.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            Error::BadRequest("x".into()).status(),
            StatusCode::BAD_REQUEST
        );
        let io = std::io::Error::other("disk");
        assert_eq!(Error::from(io).status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
