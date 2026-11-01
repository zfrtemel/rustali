//! Server-rendered blog: Askama views, a `#[derive(Entity)]` model, validation that
//! redirects back with errors and old input, flash messages and CSRF protection.
//!
//! ```text
//! cargo run -p blog -- migrate
//! cargo run -p blog            # http://127.0.0.1:8000
//! ```

use rustali::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    std::env::set_current_dir(env!("CARGO_MANIFEST_DIR"))?;
    blog::app().run().await
}
