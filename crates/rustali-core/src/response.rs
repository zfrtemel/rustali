//! [`ApiResponse`]: one fixed JSON envelope for every API response.
//!
//! Success and failure share the **same three fields** — `success`, `data`, `error` — so a
//! client parses one shape and never has to branch on which keys are present. On success
//! `error` is `null`; on failure `data` is `null`; the envelope itself never changes.
//!
//! ```json
//! { "success": true,  "data": { "id": 1 }, "error": null }
//! { "success": false, "data": null, "error": { "code": 404, "message": "Not Found", "details": null } }
//! ```
//!
//! The error-negotiation layer renders framework [`Error`](crate::Error)s into this very
//! envelope, so handler-returned successes and propagated failures look identical in shape.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::Value;

/// The error half of the envelope. Always present on a failure, always `null` on success.
#[derive(Debug, Clone, Serialize)]
pub struct ApiError {
    /// HTTP status code, repeated in the body for clients that only read JSON.
    pub code: u16,
    /// Human-readable message. Safe to show; never leaks internals.
    pub message: String,
    /// Extra machine-readable context (e.g. per-field validation errors), or `null`.
    pub details: Option<Value>,
}

/// The fixed response envelope. Serialises as `{ success, data, error }` with every field
/// always present.
#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    /// `true` for a success, `false` for a failure.
    pub success: bool,
    /// The payload on success, `null` on failure.
    pub data: Option<T>,
    /// The error on failure, `null` on success.
    pub error: Option<ApiError>,
}

impl<T> ApiResponse<T> {
    /// A success carrying `data` (HTTP 200).
    #[must_use]
    pub const fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// A failure carrying an [`ApiError`] (no data).
    #[must_use]
    pub const fn fail(error: ApiError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

impl ApiError {
    /// Build an error part from a status and message, no details.
    #[must_use]
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            code: status.as_u16(),
            message: message.into(),
            details: None,
        }
    }

    /// Attach machine-readable details (e.g. validation errors).
    #[must_use]
    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        // Status comes from the error code on failure, 200 on success.
        let status = self
            .error
            .as_ref()
            .and_then(|e| StatusCode::from_u16(e.code).ok())
            .unwrap_or(StatusCode::OK);
        (status, Json(self)).into_response()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn success_and_failure_share_the_same_three_keys() {
        let ok = serde_json::to_value(ApiResponse::ok(serde_json::json!({ "id": 1 }))).unwrap();
        let fail = serde_json::to_value(ApiResponse::<()>::fail(ApiError::new(
            StatusCode::NOT_FOUND,
            "Not Found",
        )))
        .unwrap();

        // Exactly the same keys, in both cases — the shape never varies.
        let keys = |v: &serde_json::Value| {
            let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
            k.sort();
            k
        };
        assert_eq!(keys(&ok), ["data", "error", "success"]);
        assert_eq!(keys(&fail), keys(&ok));

        assert_eq!(ok["success"], true);
        assert_eq!(ok["error"], serde_json::Value::Null);
        assert_eq!(fail["success"], false);
        assert_eq!(fail["data"], serde_json::Value::Null);
        assert_eq!(fail["error"]["code"], 404);
    }

    #[test]
    fn details_attach_to_the_error() {
        let err = ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid")
            .with_details(serde_json::json!({ "fields": { "title": ["required"] } }));
        let v = serde_json::to_value(ApiResponse::<()>::fail(err)).unwrap();
        assert_eq!(v["error"]["details"]["fields"]["title"][0], "required");
    }
}
