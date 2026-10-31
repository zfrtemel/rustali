//! Flash messages and the old-input / errors carried across a redirect.

use std::{
    collections::BTreeMap,
    convert::Infallible,
    future::{Future, ready},
};

use axum::{
    extract::{FromRequestParts, OptionalFromRequestParts},
    response::{IntoResponse, Response},
};
use rustali_core::{Error, ValidationErrors, validation::FormFeedback};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::{ERRORS_KEY, FLASH_KEY, OLD_INPUT_KEY};

/// Severity of a flash message, matching common CSS alert classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Something succeeded.
    Success,
    /// Neutral information.
    Info,
    /// A non-fatal warning.
    Warning,
    /// Something went wrong.
    Error,
}

impl Level {
    /// Lower-case class name (`success`, `info`, `warning`, `error`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One flash message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Severity.
    pub level: Level,
    /// Text shown to the user.
    pub text: String,
}

/// One-request session storage: messages, old form input and validation errors set on
/// this request survive exactly one redirect and are cleared when read.
///
/// Extracted by handlers to *write* (`flash.success(..)`) and by templates to *read*
/// (`flash.take()`, `old(..)`, `errors()`), which is why it is cheaply clonable and
/// backed by the request [`Session`].
#[derive(Debug, Clone)]
pub struct Flash {
    session: Session,
}

impl Flash {
    /// Wrap a session.
    #[must_use]
    pub const fn new(session: Session) -> Self {
        Self { session }
    }

    /// Queue a message for the next request.
    pub async fn push(&self, level: Level, text: impl Into<String>) {
        let mut messages: Vec<Message> = self
            .session
            .get(FLASH_KEY)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        messages.push(Message {
            level,
            text: text.into(),
        });
        let _ = self.session.insert(FLASH_KEY, messages).await;
    }

    /// Queue a success message.
    pub async fn success(&self, text: impl Into<String>) {
        self.push(Level::Success, text).await;
    }

    /// Queue an info message.
    pub async fn info(&self, text: impl Into<String>) {
        self.push(Level::Info, text).await;
    }

    /// Queue a warning message.
    pub async fn warning(&self, text: impl Into<String>) {
        self.push(Level::Warning, text).await;
    }

    /// Queue an error message.
    pub async fn error(&self, text: impl Into<String>) {
        self.push(Level::Error, text).await;
    }

    /// Take and clear all queued messages (call once while rendering).
    pub async fn take(&self) -> Vec<Message> {
        let messages = self
            .session
            .get(FLASH_KEY)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        let _ = self.session.remove::<Vec<Message>>(FLASH_KEY).await;
        messages
    }

    /// Validation errors flashed by a failed `Validated<Form<T>>`, if any.
    pub async fn errors(&self) -> ValidationErrors {
        let errors = self
            .session
            .get(ERRORS_KEY)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        let _ = self.session.remove::<ValidationErrors>(ERRORS_KEY).await;
        errors
    }

    /// Previously submitted value for `field` (for re-filling a form).
    pub async fn old(&self, field: &str) -> Option<String> {
        let map: BTreeMap<String, serde_json::Value> =
            self.session.get(OLD_INPUT_KEY).await.ok().flatten()?;
        match map.get(field)? {
            serde_json::Value::String(s) => Some(s.clone()),
            other => Some(other.to_string()),
        }
    }

    /// Clear the flashed old input (call after rendering a form once).
    pub async fn clear_old(&self) {
        let _ = self
            .session
            .remove::<serde_json::Value>(OLD_INPUT_KEY)
            .await;
    }
}

impl<S: Sync> FromRequestParts<S> for Flash {
    type Rejection = Response;

    fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        // The SessionManagerLayer (added by SessionProvider) inserts `Session` into
        // extensions, where tower-sessions' own extractor also reads it. Missing it
        // means the provider was not installed.
        let result = parts
            .extensions
            .get::<Session>()
            .cloned()
            .map(Self::new)
            .ok_or_else(|| {
                tracing::error!("Flash extractor used without SessionProvider");
                Error::internal(MissingSession).into_response()
            });
        ready(result)
    }
}

impl<S: Sync> OptionalFromRequestParts<S> for Flash {
    type Rejection = Infallible;

    fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Option<Self>, Self::Rejection>> + Send {
        let flash = parts.extensions.get::<Session>().cloned().map(Self::new);
        ready(Ok(flash))
    }
}

/// Raised when [`Flash`] is extracted without the session layer installed.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("no session; add SessionProvider to the application")]
pub struct MissingSession;

/// [`FormFeedback`] backed by a session: used by `Validated<Form<T>>` to flash errors
/// and old input before redirecting back.
pub struct SessionFeedback {
    session: Session,
}

impl SessionFeedback {
    pub(crate) const fn new(session: Session) -> Self {
        Self { session }
    }
}

impl FormFeedback for SessionFeedback {
    fn remember<'a>(
        &'a self,
        errors: &'a ValidationErrors,
        old_input: &'a serde_json::Value,
    ) -> rustali_core::validation::FeedbackFuture<'a> {
        Box::pin(async move {
            let _ = self.session.insert(ERRORS_KEY, errors).await;
            let _ = self.session.insert(OLD_INPUT_KEY, old_input).await;
        })
    }
}
