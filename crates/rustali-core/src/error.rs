//! The framework-wide error type and its HTTP representation.
//!
//! Every fallible framework API returns [`Result<T>`]. Handlers return it too. When an
//! [`Error`] is converted into a response it carries an [`ErrorBody`] extension; the
//! [`crate::middleware::negotiate_errors`] layer (part of the default stack) reads it
//! and renders either an RFC 9457 *Problem Details* JSON document or an HTML error
//! page, depending on what the client accepts. This is the pipeline's "exception
//! filter".

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
/// leak details to clients unless `app.debug` is enabled.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The requested resource does not exist (404).
    #[error("not found")]
    NotFound,

    /// The request is syntactically invalid (400). The message is shown to clients.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// Authentication is required or failed (401).
    #[error("unauthorized")]
    Unauthorized,

    /// The caller is authenticated but not allowed (403).
    #[error("forbidden")]
    Forbidden,

    /// Rate limit exceeded (429). Rendered with a `Retry-After` header.
    #[error("too many requests; retry after {retry_after_secs}s")]
    TooManyRequests {
        /// Seconds until the client may retry.
        retry_after_secs: u64,
    },

    /// Configuration could not be loaded or is invalid (startup only).
    ///
    /// Boxed because `figment::Error` is large and would bloat every `Result<T>`.
    #[error("configuration error: {0}")]
    Config(Box<figment::Error>),

    /// A database operation failed (500). Produced by the ORM layer; `core` stays
    /// independent of any database crate.
    #[error("database error: {0}")]
    Database(Box<dyn std::error::Error + Send + Sync + 'static>),

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

    /// Build a 400 with a client-visible message.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    /// Wrap a database driver error.
    pub fn database<E>(err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Database(Box::new(err))
    }

    /// The HTTP status this error renders as.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::TooManyRequests { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::Config(_) | Self::Database(_) | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    /// Short human-readable title, e.g. `Not Found`.
    #[must_use]
    pub fn title(&self) -> &'static str {
        self.status().canonical_reason().unwrap_or("Error")
    }

    /// Detail that is safe to show to any client. Server-side failures return `None`.
    #[must_use]
    pub fn public_detail(&self) -> Option<String> {
        match self {
            Self::BadRequest(message) => Some(message.clone()),
            _ => None,
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

/// Response extension describing a failed request.
///
/// Inserted by [`Error::into_response`] and consumed by the error negotiation
/// middleware, which decides the final representation. Custom error renderers can
/// read it the same way.
#[derive(Debug, Clone)]
pub struct ErrorBody {
    /// HTTP status of the failure.
    pub status: StatusCode,
    /// Short title, e.g. `Not Found`.
    pub title: &'static str,
    /// Detail safe for every client.
    pub detail: Option<String>,
    /// Full error text; only rendered when `app.debug` is on.
    pub debug_detail: String,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(error = %self, "request failed");
        } else {
            tracing::debug!(error = %self, %status, "request rejected");
        }

        let body = ErrorBody {
            status,
            title: self.title(),
            detail: self.public_detail(),
            debug_detail: self.to_string(),
        };
        // Plain-text fallback for apps that remove the negotiation layer.
        let mut response = (status, body.title).into_response();
        if let Self::TooManyRequests { retry_after_secs } = &self
            && let Ok(value) = axum::http::HeaderValue::from_str(&retry_after_secs.to_string())
        {
            response
                .headers_mut()
                .insert(axum::http::header::RETRY_AFTER, value);
        }
        response.extensions_mut().insert(body);
        response
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
        assert_eq!(Error::bad_request("x").status(), StatusCode::BAD_REQUEST);
        let io = std::io::Error::other("disk");
        assert_eq!(Error::from(io).status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn only_client_errors_expose_detail() {
        assert_eq!(
            Error::bad_request("missing id").public_detail().as_deref(),
            Some("missing id")
        );
        assert!(
            Error::internal(std::io::Error::other("secret"))
                .public_detail()
                .is_none()
        );
    }

    #[test]
    fn response_carries_error_body_extension() {
        let response = Error::NotFound.into_response();
        let body = response.extensions().get::<ErrorBody>().cloned();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(body.map(|b| b.title), Some("Not Found"));
    }
}
