//! Input validation for the Rustali web framework.
//!
//! Declare rules on your DTO, then ask for `Validated<Form<T>>` or
//! `Validated<Json<T>>` in the handler. Invalid JSON requests get a 422 response in the
//! standard envelope with the field errors under `error.details.fields`; invalid HTML form
//! posts are redirected back with the errors and the old input flashed into the session (when
//! the session package is installed).
//!
//! ```ignore
//! #[derive(Deserialize, Validate)]
//! struct CreatePost {
//!     #[rule(required, length(min = 3, max = 200))]
//!     title: String,
//!     #[rule(email)]
//!     author_email: String,
//!     #[rule(range(min = 1, max = 5))]
//!     rating: u8,
//!     #[rule(confirmed)]
//!     password: String,
//!     password_confirmation: String,
//! }
//!
//! async fn store(Validated(Form(input)): Validated<Form<CreatePost>>) -> Result<Redirect> { .. }
//! ```

pub mod extract;
pub mod rules;

pub use rustali_core::ValidationErrors;

pub use extract::Validated;

/// Implemented by `#[derive(Validate)]`, or by hand for custom logic.
pub trait Validate {
    /// Check every rule; collect all failures instead of stopping at the first.
    fn validate(&self) -> Result<(), ValidationErrors>;
}

/// Paths used by `#[derive(Validate)]` output. Not public API.
#[doc(hidden)]
pub mod __private {
    pub use rustali_core::ValidationErrors;

    pub use crate::{Validate, rules};
}
