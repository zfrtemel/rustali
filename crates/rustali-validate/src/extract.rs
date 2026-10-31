//! `Validated<Form<T>>` and `Validated<Json<T>>`.
//!
//! The extractor rejection *is* a `Response` (a redirect or a problem document), so
//! `Result<_, Response>` here is intentional and the large-error lint does not apply.
#![allow(clippy::result_large_err)]

use std::ops::Deref;

use axum::{
    Form, Json,
    body::Bytes,
    extract::{FromRequest, Request},
    http::{header, request::Parts},
    response::{IntoResponse, Redirect, Response},
};
use rustali_core::{Error, middleware::wants_json, validation::FormFeedbackHandle};
use serde::de::DeserializeOwned;

use crate::{Validate, ValidationErrors};

/// Largest request body the validated extractors accept.
pub const BODY_LIMIT: usize = 2 * 1024 * 1024;

/// Extract, deserialise and validate in one step.
///
/// On failure:
/// - JSON clients (or `/api/*` paths) get `422 Unprocessable Entity` as a problem
///   document with an `errors` object;
/// - HTML form posts are redirected back (`Referer`, else `/`) after the errors and
///   the submitted values were handed to the session package through
///   [`FormFeedbackHandle`], so the form re-renders filled in with messages. Without
///   a session, they also get the 422 page.
#[derive(Debug, Clone)]
pub struct Validated<E>(pub E);

impl<E> Deref for Validated<E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Validated<Form<T>> {
    /// The validated value.
    pub fn into_inner(self) -> T {
        self.0.0
    }
}

impl<T> Validated<Json<T>> {
    /// The validated value.
    pub fn into_inner(self) -> T {
        self.0.0
    }
}

impl<T, S> FromRequest<S> for Validated<Form<T>>
where
    T: DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(request: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let (parts, body) = request.into_parts();
        let bytes = read_body(body).await?;

        let value: T = serde_urlencoded::from_bytes(&bytes).map_err(|err| {
            Error::bad_request(format!("invalid form data: {err}")).into_response()
        })?;

        match value.validate() {
            Ok(()) => Ok(Self(Form(value))),
            Err(errors) => {
                let old = old_input_from_form(&bytes);
                Err(reject(&parts, errors, old).await)
            }
        }
    }
}

impl<T, S> FromRequest<S> for Validated<Json<T>>
where
    T: DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(request: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let (parts, body) = request.into_parts();
        let bytes = read_body(body).await?;

        let value: T = serde_json::from_slice(&bytes)
            .map_err(|err| Error::bad_request(format!("invalid JSON: {err}")).into_response())?;

        match value.validate() {
            Ok(()) => Ok(Self(Json(value))),
            Err(errors) => {
                let old = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
                Err(reject(&parts, errors, old).await)
            }
        }
    }
}

async fn read_body(body: axum::body::Body) -> Result<Bytes, Response> {
    axum::body::to_bytes(body, BODY_LIMIT)
        .await
        .map_err(|err| Error::bad_request(format!("could not read body: {err}")).into_response())
}

/// `a=1&a=2&b=x` → `{"a": ["1", "2"], "b": "x"}`.
fn old_input_from_form(bytes: &[u8]) -> serde_json::Value {
    let pairs: Vec<(String, String)> = serde_urlencoded::from_bytes(bytes).unwrap_or_default();
    let mut map = serde_json::Map::new();
    for (key, value) in pairs {
        match map.get_mut(&key) {
            None => {
                map.insert(key, serde_json::Value::String(value));
            }
            Some(serde_json::Value::Array(list)) => list.push(serde_json::Value::String(value)),
            Some(existing) => {
                let first = existing.take();
                *existing = serde_json::Value::Array(vec![first, serde_json::Value::String(value)]);
            }
        }
    }
    serde_json::Value::Object(map)
}

/// Build the failure response for `parts`.
pub async fn reject(parts: &Parts, errors: ValidationErrors, old: serde_json::Value) -> Response {
    let json = wants_json(&parts.headers, parts.uri.path());
    if !json && let Some(feedback) = parts.extensions.get::<FormFeedbackHandle>() {
        feedback.0.remember(&errors, &old).await;
        let back = parts
            .headers
            .get(header::REFERER)
            .and_then(|v| v.to_str().ok())
            .filter(|v| v.starts_with('/') || v.starts_with("http"))
            .unwrap_or("/");
        tracing::debug!(%errors, back, "form invalid; redirecting back");
        return Redirect::to(back).into_response();
    }
    Error::Validation(errors).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_input_keeps_repeated_fields() {
        let value = old_input_from_form(b"title=Hi&tags=a&tags=b");
        assert_eq!(value["title"], "Hi");
        assert_eq!(value["tags"], serde_json::json!(["a", "b"]));
    }
}
