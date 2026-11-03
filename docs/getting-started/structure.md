# Project Structure

`rustali new` scaffolds a small, conventional layout. Nothing here is magic — it is
ordinary Rust, and you can rename or rearrange it — but following the convention is what
lets the generators and `App::builder().controllers()` find your code.

## A fresh project

```text
blog/
├── Cargo.toml
├── .env                  # all configuration (one file)
├── db/
│   └── .gitkeep          # the SQLite file, migrations and snapshot live here
├── src/
│   ├── main.rs           # App::builder()...run()
│   └── controllers.rs    # a home controller: "/" and "/api/ping"
└── templates/
    └── home.html         # fullstack preset only
```

`templates/` and `home.html` are generated only by the `fullstack` preset, and
`src/models.rs` + `src/migrations.rs` appear only when the `auth` package is selected
(which `fullstack` pre-checks).

What each top-level piece is for:

- **`.env`** — all configuration in one file: `APP_*`, `DATABASE_URL`, and a line per optional
  package. See [Configuration](configuration). Real secrets come from the environment in
  production, overriding `.env`.
- **`db/`** — the SQLite file (in development), your migrations, and the schema snapshot that
  drives `migrate:diff`.
- **`src/`** — your code. `main.rs` boots the app; everything else is the controllers,
  models and so on that the generators create.
- **`templates/`** — Askama templates, compiled into the binary (present only in the
  `fullstack` preset).

## `main.rs`

The entry point is deliberately tiny — it names the providers your app uses and mounts
every controller:

```rust
mod controllers;

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

`run()` doubles as the console entry point: with no arguments it serves; with a command
(`migrate`, `routes`, `queue:work`, ...) it runs that instead. See
[Console Commands](../deeper/console).

## Where generated code goes

`rustali make:<thing> <Name>` writes into conventional subdirectories of `src/`:

| Command | Writes to |
|---|---|
| `make:controller` | `src/controllers/` |
| `make:model` | `src/models/` |
| `make:migration` | `src/migrations/` |
| `make:job` | `src/jobs/` |
| `make:mail` | `src/mail/` |
| `make:middleware` | `src/middleware/` |
| `make:policy` | `src/policies/` |
| `make:test` | `tests/` |

A generator never overwrites an existing file unless you pass `--force`. After
`make:controller Post`, declare the module (`mod controllers;` with `pub mod post;`) so
it compiles; the controller then auto-mounts through `controllers()`.

## Growing beyond one file

As an app grows, a lib + bin split is useful — it lets integration tests build the same
application `main` does. The `blog` example does exactly this:

```rust
// src/lib.rs
pub mod controllers;
pub mod migrations;
pub mod models;

pub fn app() -> AppBuilder {
    App::builder()
        .provider(DatabaseProvider)
        .provider(SessionProvider)
        .migrations::<migrations::Migrator>()
        .controllers()
        .configure(session_layers)
}

// src/main.rs
#[tokio::main]
async fn main() -> Result<()> {
    blog::app().run().await
}
```

Now a test can call `blog::app()` and drive it with the [`TestClient`](../deeper/testing)
— the same providers, routes and middleware as production.
