//! CSRF protection for state-changing form requests.
//!
//! A random token is stored in the session and must accompany every unsafe request
//! (`POST`, `PUT`, `PATCH`, `DELETE`) as the `_token` form field or the `X-CSRF-Token`
//! header. Safe methods and `/api/*` paths (which use bearer tokens, not cookies) are
//! exempt. Templates embed the token with `csrf.field()`.

use std::{
    future::{Future, ready},
    pin::Pin,
    task::{Context, Poll},
};

use axum::{
    extract::{FromRequestParts, Request},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rustali_core::Error;
use subtle::ConstantTimeEq;
use tower::{Layer, Service};
use tower_sessions::Session;

use crate::CSRF_KEY;

/// Reads or creates the per-session CSRF token and renders form helpers.
#[derive(Debug, Clone)]
pub struct Csrf {
    session: Session,
}

impl Csrf {
    /// Wrap a session.
    #[must_use]
    pub const fn new(session: Session) -> Self {
        Self { session }
    }

    /// Current token, creating and storing one if absent.
    pub async fn token(&self) -> String {
        if let Some(token) = self.session.get::<String>(CSRF_KEY).await.ok().flatten() {
            return token;
        }
        let token = generate();
        let _ = self.session.insert(CSRF_KEY, token.clone()).await;
        token
    }

    /// A hidden input carrying the token, for embedding in a `<form>`.
    pub async fn field(&self) -> String {
        format!(
            r#"<input type="hidden" name="_token" value="{}">"#,
            self.token().await
        )
    }
}

impl<S: Sync> FromRequestParts<S> for Csrf {
    type Rejection = Response;

    fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let result = parts
            .extensions
            .get::<Session>()
            .cloned()
            .map(Self::new)
            .ok_or_else(|| Error::internal(crate::flash::MissingSession).into_response());
        ready(result)
    }
}

fn generate() -> String {
    let bytes: [u8; 32] = rand::random();
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Compare two tokens in constant time.
#[must_use]
pub fn tokens_match(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

/// Layer enforcing CSRF on unsafe, cookie-based requests.
#[derive(Debug, Clone, Default)]
pub struct CsrfLayer;

impl<S> Layer<S> for CsrfLayer {
    type Service = CsrfService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CsrfService { inner }
    }
}

/// Service produced by [`CsrfLayer`].
#[derive(Debug, Clone)]
pub struct CsrfService<S> {
    inner: S,
}

const fn is_safe(method: &Method) -> bool {
    matches!(
        *method,
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    )
}

impl<S> Service<Request> for CsrfService<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> <Self as Service<Request>>::Future {
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        // Exempt safe methods and API routes (token-authenticated, no cookies).
        if is_safe(request.method()) || request.uri().path().starts_with("/api/") {
            return Box::pin(async move { inner.call(request).await });
        }

        Box::pin(async move {
            let Some(session) = request.extensions().get::<Session>().cloned() else {
                // No session layer → nothing to protect against; let it through.
                return inner.call(request).await;
            };
            let expected = session.get::<String>(CSRF_KEY).await.ok().flatten();

            let (parts, body) = request.into_parts();
            let bytes = match axum::body::to_bytes(body, 2 * 1024 * 1024).await {
                Ok(bytes) => bytes,
                Err(err) => {
                    return Ok(
                        Error::bad_request(format!("could not read body: {err}")).into_response()
                    );
                }
            };

            let presented = parts
                .headers
                .get("x-csrf-token")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
                .or_else(|| token_from_form(&parts.headers, &bytes));

            match (expected, presented) {
                (Some(expected), Some(presented)) if tokens_match(&expected, &presented) => {
                    let request = Request::from_parts(parts, axum::body::Body::from(bytes));
                    inner.call(request).await
                }
                _ => Ok((StatusCode::FORBIDDEN, "CSRF token missing or invalid").into_response()),
            }
        })
    }
}

/// Pull `_token` from a urlencoded body (only when the content type says so).
fn token_from_form(headers: &axum::http::HeaderMap, bytes: &[u8]) -> Option<String> {
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)?
        .to_str()
        .ok()?;
    if !content_type.starts_with("application/x-www-form-urlencoded") {
        return None;
    }
    serde_urlencoded::from_bytes::<Vec<(String, String)>>(bytes)
        .ok()?
        .into_iter()
        .find(|(k, _)| k == "_token")
        .map(|(_, v)| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_compare_in_constant_time() {
        let token = generate();
        assert!(tokens_match(&token, &token));
        assert!(!tokens_match(&token, &generate()));
        assert!(!tokens_match("a", "ab"));
    }

    #[test]
    fn generated_tokens_are_urlsafe_and_long() {
        let token = generate();
        assert!(token.len() >= 43, "32 bytes base64url");
        assert!(
            token
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }

    #[test]
    fn safe_methods_are_exempt() {
        assert!(is_safe(&Method::GET));
        assert!(!is_safe(&Method::POST));
    }
}
