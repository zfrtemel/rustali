//! A JSON API backed by SQLite: migrations, a `#[derive(Entity)]` model and an
//! attribute-routed controller.
//!
//! ```text
//! cargo run -p todo-api -- migrate          # create the schema
//! cargo run -p todo-api -- routes           # list the controller routes
//! cargo run -p todo-api                      # serve on http://127.0.0.1:8000
//!
//! curl -X POST localhost:8000/api/todos -H 'content-type: application/json' -d '{"title":"ship it"}'
//! curl localhost:8000/api/todos?page=1&per_page=20
//! curl -X PATCH localhost:8000/api/todos/1 -H 'content-type: application/json' -d '{"done":true}'
//! curl -X DELETE localhost:8000/api/todos/1
//! ```

mod controllers;
mod migrations;
mod models;

use rustali::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Examples live inside the workspace; run from their own directory so relative
    // paths in config/database.toml resolve the same way a real app's would.
    std::env::set_current_dir(env!("CARGO_MANIFEST_DIR"))?;

    App::builder()
        .provider(DatabaseProvider)
        .migrations::<migrations::Migrator>()
        .controllers()
        .run()
        .await
}
