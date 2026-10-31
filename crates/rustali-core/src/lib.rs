//! Application kernel for the Rustali web framework.
//!
//! `rustali-core` owns the pieces every other crate builds on: the [`App`] builder
//! and its shared [`AppContext`], layered [`Config`] loading, explicit dependency
//! injection ([`Provider`], [`Services`], [`Inject`]), the framework-wide [`Error`]
//! type with content-negotiated responses, the default middleware stack, the
//! controller registry behind `#[controller]`, authorization ([`auth`]) and the
//! request extractors such as [`Ctx`].
//!
//! End users normally depend on the `rustali` meta crate instead of this one.
//!
//! # Example
//!
//! ```no_run
//! use rustali_core::{App, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     App::builder().serve().await
//! }
//! ```

pub mod app;
pub mod auth;
pub mod command;
pub mod config;
pub mod error;
pub mod extract;
pub mod middleware;
pub mod router;
pub mod services;
pub mod telemetry;

pub use app::{App, AppBuilder, AppContext};
pub use command::{Command, CommandFuture, Commands};
pub use config::Config;
pub use error::{Error, ErrorBody, Result};
pub use extract::{Ctx, Inject};
pub use services::{MissingService, Provider, RegisterFuture, Registry, Services};

/// Paths used by generated code. Not part of the public API; may change at any time.
#[doc(hidden)]
pub mod __private {
    pub use axum;
    pub use inventory;

    pub use crate::{
        AppContext,
        auth::AuthorizeLayer,
        router::{Controller, Registration, RouteDef},
    };
}
