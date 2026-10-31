//! The [`View`] response wrapper over an Askama template.

use axum::{
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use rustali_core::Error;

/// A renderable Askama template plus response metadata (status, extra headers).
///
/// Implements [`IntoResponse`]: rendering failure becomes a 500 through the framework
/// error type, so a broken template is logged and shown like any other server error.
#[cfg(feature = "askama")]
pub struct View<T: askama::Template> {
    template: T,
    status: StatusCode,
}

#[cfg(feature = "askama")]
impl<T: askama::Template> View<T> {
    /// A 200 response rendering `template`.
    #[must_use]
    pub const fn new(template: T) -> Self {
        Self {
            template,
            status: StatusCode::OK,
        }
    }

    /// Set a non-200 status (e.g. `422` for a re-rendered form).
    #[must_use]
    pub const fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Render to an HTML string now, returning a framework error on failure.
    pub fn render(&self) -> Result<String, Error> {
        self.template
            .render()
            .map_err(ViewError::from)
            .map_err(Error::internal)
    }
}

#[cfg(feature = "askama")]
impl<T: askama::Template> IntoResponse for View<T> {
    fn into_response(self) -> Response {
        match self.template.render() {
            Ok(html) => {
                let mut response = (self.status, Html(html)).into_response();
                response
                    .headers_mut()
                    .entry(header::CONTENT_TYPE)
                    .or_insert(header::HeaderValue::from_static("text/html; charset=utf-8"));
                response
            }
            Err(err) => Error::internal(ViewError::from(err)).into_response(),
        }
    }
}

/// Error rendering a view.
#[derive(Debug, thiserror::Error)]
#[error("view rendering failed: {0}")]
pub struct ViewError(String);

#[cfg(feature = "askama")]
impl From<askama::Error> for ViewError {
    fn from(err: askama::Error) -> Self {
        Self(err.to_string())
    }
}
