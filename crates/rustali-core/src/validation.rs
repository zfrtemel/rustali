//! Validation results shared by the kernel, the validation package and the session
//! package.
//!
//! The kernel only defines the *shape* of a validation failure ([`ValidationErrors`])
//! and how a failed form is remembered for the next request ([`FormFeedback`]). Rules
//! and the `#[derive(Validate)]` macro live in `rustali-validate`; storing errors and
//! old input in the session lives in `rustali-session`. Keeping the contract here lets
//! both packages cooperate without depending on each other.

use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc};

use serde::{Deserialize, Serialize};

/// Field → messages. Fields keep insertion order of first failure per field name
/// (a `BTreeMap` keeps rendering deterministic).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationErrors {
    /// Messages per field. Use `"_"` for errors not tied to a field.
    pub fields: BTreeMap<String, Vec<String>>,
}

impl ValidationErrors {
    /// No errors.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a message for `field`.
    pub fn add(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.fields
            .entry(field.into())
            .or_default()
            .push(message.into());
    }

    /// Merge another set into this one.
    pub fn merge(&mut self, other: Self) {
        for (field, messages) in other.fields {
            self.fields.entry(field).or_default().extend(messages);
        }
    }

    /// `true` when nothing failed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Total number of messages.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fields.values().map(Vec::len).sum()
    }

    /// `true` when `field` has at least one message.
    #[must_use]
    pub fn has(&self, field: &str) -> bool {
        self.fields.get(field).is_some_and(|m| !m.is_empty())
    }

    /// First message for `field`, handy in templates.
    #[must_use]
    pub fn first(&self, field: &str) -> Option<&str> {
        self.fields
            .get(field)
            .and_then(|m| m.first())
            .map(String::as_str)
    }

    /// All messages for `field`.
    #[must_use]
    pub fn get(&self, field: &str) -> &[String] {
        self.fields.get(field).map_or(&[], Vec::as_slice)
    }

    /// Every message, flattened, in field order.
    pub fn all(&self) -> impl Iterator<Item = &str> {
        self.fields.values().flatten().map(String::as_str)
    }

    /// `Ok(())` when empty, otherwise `Err(self)`.
    pub fn into_result(self) -> Result<(), Self> {
        if self.is_empty() { Ok(()) } else { Err(self) }
    }
}

impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for (field, messages) in &self.fields {
            for message in messages {
                if !first {
                    f.write_str("; ")?;
                }
                first = false;
                write!(f, "{field}: {message}")?;
            }
        }
        Ok(())
    }
}

/// Future returned by [`FormFeedback::remember`].
pub type FeedbackFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

/// Remembers a failed form submission (errors + old input) for the next request, so
/// an SSR page can redirect back and re-render the form filled in.
///
/// Implemented by the session package and placed in request extensions as
/// [`FormFeedbackHandle`]; consumed by the `Validated<Form<T>>` extractor.
pub trait FormFeedback: Send + Sync + 'static {
    /// Store `errors` and `old_input` until the next request.
    fn remember<'a>(
        &'a self,
        errors: &'a ValidationErrors,
        old_input: &'a serde_json::Value,
    ) -> FeedbackFuture<'a>;
}

/// Request extension carrying the active [`FormFeedback`] implementation.
#[derive(Clone)]
pub struct FormFeedbackHandle(pub Arc<dyn FormFeedback>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_and_reports() {
        let mut errors = ValidationErrors::new();
        assert!(errors.is_empty());
        errors.add("title", "The title field is required.");
        errors.add("title", "The title must be at least 3 characters.");
        errors.add("email", "The email must be a valid email address.");
        assert_eq!(errors.len(), 3);
        assert!(errors.has("title") && !errors.has("body"));
        assert_eq!(errors.first("title"), Some("The title field is required."));
        assert_eq!(errors.get("email").len(), 1);
        assert_eq!(errors.all().count(), 3);
        assert!(errors.to_string().starts_with("email: "));
        assert!(errors.clone().into_result().is_err());
        assert!(ValidationErrors::new().into_result().is_ok());
    }

    #[test]
    fn merge_appends() {
        let mut a = ValidationErrors::new();
        a.add("x", "1");
        let mut b = ValidationErrors::new();
        b.add("x", "2");
        b.add("y", "3");
        a.merge(b);
        assert_eq!(a.get("x"), ["1", "2"]);
        assert_eq!(a.get("y"), ["3"]);
    }
}
