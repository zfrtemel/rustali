//! Blog example as a library, so integration tests can reuse the real app.

pub mod controllers;
pub mod migrations;
pub mod models;

use rustali::{AppBuilder, prelude::*};

/// Assemble the application. Shared by `main` and the integration tests so they exercise
/// the exact same middleware stack, providers and routes.
#[must_use]
pub fn app() -> AppBuilder {
    App::builder()
        .provider(DatabaseProvider)
        .provider(SessionProvider)
        .migrations::<migrations::Migrator>()
        .controllers()
        // Session + CSRF layers need runtime config, so they are added in `configure`.
        .configure(session_layers)
}
