//! Application kernel for the Rustali web framework.
//!
//! `rustali-core` owns the pieces every other crate builds on: the [`App`] builder
//! and its shared [`AppContext`], layered [`Config`] loading, the framework-wide
//! [`Error`] type with content-negotiated responses, and the request extractors.
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
pub mod config;
pub mod error;

pub use app::{App, AppBuilder, AppContext};
pub use config::Config;
pub use error::{Error, Result};
