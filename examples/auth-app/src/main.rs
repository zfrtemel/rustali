//! The batteries-included auth flow: register, login, logout and a protected page,
//! with argon2 hashing, cookie sessions, CSRF and validation — all wired in `main`.
//!
//! ```text
//! cargo run -p auth-app -- migrate
//! cargo run -p auth-app            # http://127.0.0.1:8000/register
//! ```

mod controllers;
mod migrations;
mod models;

use rustali::prelude::*;

use crate::models::Users;

#[tokio::main]
async fn main() -> Result<()> {
    std::env::set_current_dir(env!("CARGO_MANIFEST_DIR"))?;

    App::builder()
        .provider(DatabaseProvider)
        .provider(SessionProvider)
        .provider(AuthProvider::new(Users))
        .migrations::<migrations::Migrator>()
        .controllers()
        .configure(session_layers)
        .run()
        .await
}
