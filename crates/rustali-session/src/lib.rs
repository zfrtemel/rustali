//! Sessions, flash messages, CSRF protection and form feedback.
//!
//! Add [`SessionProvider`] to the app and you get:
//!
//! - cookie-backed sessions (via [`tower_sessions`]), signed with `app.key`;
//! - the [`Session`] extractor re-exported for handlers;
//! - [`Flash`] one-request messages (`flash.success("Saved.")`);
//! - CSRF protection for state-changing form posts ([`Csrf`] token + verifying layer);
//! - the [`FormFeedback`](rustali_core::validation::FormFeedback) implementation that
//!   lets `Validated<Form<T>>` redirect back with errors and old input.
//!
//! ```ignore
//! App::builder().provider(SessionProvider::default()).controllers().run().await
//! ```

pub mod csrf;
pub mod flash;
pub mod provider;

pub use flash::Message;

pub use tower_sessions::Session;

pub use csrf::{Csrf, CsrfLayer};
pub use flash::Flash;
pub use provider::{SessionConfig, SessionProvider, layers as session_layers};

/// Session key under which framework data is namespaced, to avoid clashing with
/// application keys.
pub(crate) const FLASH_KEY: &str = "_rustali.flash";
pub(crate) const OLD_INPUT_KEY: &str = "_rustali.old";
pub(crate) const ERRORS_KEY: &str = "_rustali.errors";
pub(crate) const CSRF_KEY: &str = "_rustali.csrf";
