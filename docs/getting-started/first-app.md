# Your First App

```bash
cargo install rustali-cli     # once 0.1.0 is published
rustali new blog
cd blog
cargo run -- migrate          # create the SQLite schema
cargo run                     # http://127.0.0.1:8000
```

Open `http://127.0.0.1:8000` for the HTML home page, and the scaffold also ships a JSON
route you can hit right away:

```bash
curl http://127.0.0.1:8000/api/ping
# {"ok":true}
```

That one project already shows the core idea — the same app serves an HTML page and a JSON
API from one controller.

## What you get

`rustali new` scaffolds a conventional project:

```text
blog/
├── Cargo.toml
├── .env                  # all configuration: name, env, debug, key, DATABASE_URL, …
├── db/                   # SQLite file + migrations + schema snapshot
├── src/
│   ├── main.rs           # App::builder()...run()  (≈ 8 lines)
│   └── controllers.rs    # a home controller: SSR "/" and JSON "/api/ping"
└── templates/
    └── home.html
```

`src/main.rs` is the whole bootstrap — a builder, the database provider, and the controllers
auto-mounted:

```rust
use rustali::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    App::builder()
        .provider(DatabaseProvider)
        .controllers()
        .run()
        .await
}
```

And `src/controllers.rs` is the home controller it mounts — note the two routes, one HTML and
one JSON, side by side:

```rust
#[controller("/")]
impl HomeController {
    #[get("/", name = "home")]
    async fn home() -> &'static str {
        "Welcome to Rustali!"
    }

    #[get("/api/ping", name = "ping")]
    async fn ping() -> Json<Ping> {
        Json(Ping { ok: true })
    }
}
```

## Add a resource

```bash
rustali make:model Post          # src/models/post.rs  (#[derive(Entity)])
rustali make:controller Post     # src/controllers/post.rs
rustali make:migration create_posts
```

Edit the model's columns to describe your table:

```rust
// src/models/post.rs
use rustali::{orm::chrono::{DateTime, Utc}, prelude::*};
use serde::Serialize;

#[derive(Entity, Debug, Clone, Serialize)]
#[table("posts")]
pub struct Post {
    pub id: i64,
    #[column(max_len = 200)]
    pub title: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Then generate the migration from the change — the diff reads your entity and writes the
first draft:

```bash
cargo run -- migrate:diff "create posts"   # writes a reviewed migration + snapshot
cargo run -- migrate
```

::: tip
`migrate:diff` writes the generated migration under `db/migrations/`. Move it into
`src/migrations/` and list it in your `Migrator` so it compiles — automatic placement is
on the roadmap. See [Migrations](../database/migrations) for the full workflow.
:::

Mount the controller (`App::builder().controllers()` already picks up every
`#[controller]` in the binary) and you have CRUD. See
[Routing & Controllers](../basics/routing) next.
