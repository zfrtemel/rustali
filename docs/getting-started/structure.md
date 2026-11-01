# Project Structure

`rustali new` scaffolds a small, conventional layout. Nothing here is magic — it is
ordinary Rust, and you can rename or rearrange it — but following the convention is what
lets the generators and `App::builder().controllers()` find your code.

## A fresh project

```text
blog/
├── Cargo.toml
├── .env                  # local environment (git-ignored)
├── config/
│   ├── app.toml          # name, env, debug, key
│   └── database.toml     # connection URL (SQLite by default)
├── db/
│   └── .gitkeep          # the SQLite file, migrations and snapshot live here
├── src/
│   ├── main.rs           # App::builder()...run()
│   └── controllers.rs    # a home controller: "/" and "/api/ping"
└── templates/
    └── home.html         # omitted with `rustali new --api`
```

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
