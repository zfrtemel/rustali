# Installation

## Requirements

- **Rust** 1.97 or newer (2024 edition). Install via [rustup](https://rustup.rs).
- On **Windows**, the MSVC toolchain needs the *Visual Studio Build Tools* with the
  "Desktop development with C++" workload (it provides `link.exe`).
- A database is optional to start — the default is **SQLite**, which needs nothing
  installed. PostgreSQL and MySQL are supported too.

## With the CLI (recommended)

```bash
cargo install rustali-cli
rustali new blog
cd blog
cargo run -- migrate    # create the SQLite schema
cargo run               # serve on http://127.0.0.1:8000
```

`rustali new` scaffolds a conventional project; see [Your First App](first-app) for a
tour of what it generates. Add `--api` for a JSON-only project without the view layer.

## Into an existing crate

Add Rustali and an async runtime:

```toml
# Cargo.toml
[dependencies]
rustali = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde = { version = "1", features = ["derive"] }
```

A minimal `main.rs`:

```rust
use rustali::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    App::builder().controllers().run().await
}
```

## Feature flags

The `rustali` meta crate turns each subsystem on with a Cargo feature. The defaults
cover a typical full-stack app; trim them for a leaner build.

| Feature | Default | Brings in |
|---|---|---|
| `macros` | ✅ | `#[controller]`, `#[derive(Entity)]`, `#[derive(Validate)]` |
| `orm` | ✅ | The ORM, migrations, the `Db` extractor |
| `sqlite` | ✅ | SQLite driver |
| `postgres` | | PostgreSQL driver |
| `mysql` | | MySQL driver |
| `views` | ✅ | `View<T>`, Askama backend |
| `tera` | | Tera (runtime) view backend |
| `sessions` | ✅ | Cookie sessions, flash, CSRF |
| `validation` | ✅ | Validation rules and `Validated<T>` |
| `auth` | ✅ | Password hashing, guards, `Auth`/`Login` (implies `sessions`) |
| `cache` | ✅ | The `Cache` trait and memory store |
| `redis` | | Redis-backed cache store |
| `mail` | ✅ | `Mailable` and transports |
| `queue` | ✅ | Background jobs and a worker |
| `test` | | The in-process `TestClient` (put it in `dev-dependencies`) |

For a JSON API on Postgres with no views, for example:

```toml
[dependencies]
rustali = { version = "0.1", default-features = false, features = ["macros", "orm", "postgres", "auth", "validation"] }
```

## Editor setup

Install [rust-analyzer](https://rust-analyzer.github.io/). Point its check command at
clippy for the same lints the framework uses:

```json
// .vscode/settings.json
{ "rust-analyzer.check.command": "clippy" }
```

Next: [Your First App](first-app).
