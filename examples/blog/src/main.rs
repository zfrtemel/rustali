//! Server-rendered blog: Askama views, a `#[derive(Entity)]` model, validation that
//! redirects back with errors and old input, flash messages and CSRF protection.
//!
//! ```text
//! cargo run -p blog -- migrate
//! cargo run -p blog            # http://127.0.0.1:8000
//! ```

mod controllers;
mod migrations;
mod models;

use rustali::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    std::env::set_current_dir(env!("CARGO_MANIFEST_DIR"))?;

    App::builder()
        .provider(DatabaseProvider)
        .provider(SessionProvider)
        .migrations::<migrations::Migrator>()
        .controllers()
        // Session + CSRF layers need runtime config, so they are added in `configure`.
        .configure(session_layers)
        .run()
        .await
}
