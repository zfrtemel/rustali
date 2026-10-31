//! # Rustali
//!
//! A batteries-included, full-stack web framework for Rust. Add this one crate and
//! get HTTP routing, an ORM with code-first migrations, type-safe views, auth,
//! background jobs, mail and a CLI that all fit together.
//!
//! Most programs only need the prelude:
//!
//! ```no_run
//! use rustali::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     App::builder().serve().await
//! }
//! ```

pub use rustali_core::{App, AppBuilder, AppContext, Config, Error, Result};

// `#[cfg(feature = "macros")] pub use rustali_macros::*;` is added together with the
// first macro (Faz 2); re-exporting an empty proc-macro crate triggers lints.

/// Re-exports of the underlying HTTP stack for advanced use.
pub mod http {
    pub use axum::{Router, extract, response, routing};
}

/// Everything a typical application file imports.
pub mod prelude {
    pub use axum::{
        Json,
        extract::{Path, Query, State},
        response::{Html, IntoResponse, Redirect},
    };

    pub use crate::{App, AppContext, Config, Error, Result};
}

/// Paths used by generated code. Not part of the public API; may change at any time.
#[doc(hidden)]
pub mod __private {
    pub use axum;
    pub use rustali_core;
}
