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

`rustali new <name>` asks two questions — a **project type (preset)** and an optional set of
**packages** — and then scaffolds the project. With no flags it prompts you interactively;
flags let you skip the prompts for scripting or CI. See [Your First App](first-app) for a tour
of what a fresh project contains.

### Project type (preset)

| Preset | Gives you | Notes |
|---|---|---|
| `api` | JSON API with Swagger; `openapi` on, no views. Controller serves a text `/` and a JSON `/api/ping`. | Mounts `.routes(openapi_router())`. Pre-checks the `openapi` package. |
| `fullstack` | SSR views + sessions + `auth`; Askama views with `templates/home.html`, plus a working register/login/logout flow with its own views. | `auth` is on by default → users model + `create_users` migration + session wiring + `AuthController`. Pre-checks the `auth` package. |
| `minimal` | Bare core: the ORM and a single controller. | No views, no extra packages pre-checked. |

With no `--preset`, `rustali new` shows an interactive **Project type** selector offering the
three presets above. `--api` is shorthand for `--preset api`, and it also skips the package
prompt (as if you had passed `--no-prompt`).

### Packages

On top of a preset you can add optional packages. With neither `--with` nor `--no-prompt`
(nor `--api`), `rustali new` shows an interactive **Extra packages** multi-select with the
preset's implied package pre-checked.

| Package | Adds | `.env` |
|---|---|---|
| `cache` | In-memory cache store + provider | `CACHE_DRIVER=memory` |
| `mail` | Mail transport + provider | `MAIL_TRANSPORT=log`, `MAIL_FROM=no-reply@example.com` |
| `queue` | Background queue + worker | `QUEUE_CAPACITY=1024` |
| `auth` | Sessions + auth; `src/models/user.rs`, the `create_users` migration, `.migrations::<Migrator>()`, `.configure(session_layers)`, and a working auth controller — SSR register/login/logout + views when the preset has views, otherwise a JSON API with JWT bearer tokens (`/api/register`, `/api/login`, `/api/me`) | `SESSION_CSRF=true` |
| `openapi` | OpenAPI/Swagger router | `OPENAPI_TITLE=API` |

Each package turns on a Cargo feature on the `rustali` dependency, adds a `.provider(...)` or
`.routes(...)` line in `main.rs`, and writes its `.env` line(s). `auth` additionally generates
the user model/migration, the session wiring, and a ready-to-run `src/controllers/auth.rs`
(SSR with views, or JSON + JWT when the preset has no views). A preset pre-checks the package it
implies (`api` → `openapi`, `fullstack` → `auth`).

### Database

`rustali new` also asks which database to use. The default is **SQLite**, which needs nothing
installed; PostgreSQL and MySQL are also offered. The choice sets two things for you: the driver
Cargo feature on `rustali` and the active `DATABASE_URL` in `.env`.

| Database | `DATABASE_URL` | Cargo feature |
|---|---|---|
| `sqlite` (default) | `sqlite://db/app.db?mode=rwc` | `sqlite` (a default feature) |
| `postgres` | `postgres://postgres:postgres@localhost:5432/app` | `postgres` |
| `mysql` | `mysql://root:root@localhost:3306/app` | `mysql` |

Pick it interactively from the **Database** selector, or non-interactively with
`--database <sqlite|postgres|mysql>`. SQLite rides the crate's default features, so its
`Cargo.toml` stays `rustali = { version = "0.1" }`. Any other driver turns the defaults off and
lists everything explicitly, so only that one driver compiles:

```toml
rustali = { version = "0.1", default-features = false, features = ["macros", "orm", "postgres", "views", "sessions", "validation", "auth", "cache", "mail", "queue"] }
```

Whichever you choose, the generated `.env` keeps the other two as commented examples — each with
the one-line Cargo change — so switching later is a two-line edit (the `DATABASE_URL` and the
driver feature). Postgres and MySQL talk to a running server, so start it and point
`DATABASE_URL` at it before `rustali migrate`.

### Interactive vs. flags

- **Interactive** (no relevant flags): a **Project type** `Select`, an **Extra packages**
  `MultiSelect` with the preset's package pre-checked, then a **Database** `Select`.
- **Non-interactive** (scripting/CI): pass `--preset <api|fullstack|minimal>` to pick the type,
  `--with pkg1,pkg2` to choose packages, `--database <sqlite|postgres|mysql>` to pick the
  database, and/or `--no-prompt` to skip any remaining prompt. `--api` implies `--no-prompt`;
  with `--no-prompt` (and no `--database`) the database defaults to SQLite.

```bash
rustali new blog                                 # interactive: asks project type, then packages
rustali new api --preset api                     # api preset; still prompts for extra packages
rustali new api --api                            # same preset, but skips the package prompt
rustali new shop --preset fullstack              # SSR views + sessions + auth
rustali new svc --preset minimal --with cache,queue  # bare core plus cache and queue
rustali new shop --preset fullstack --database postgres  # SSR app on PostgreSQL
rustali new svc --preset minimal --no-prompt     # bare core, SQLite, no prompts at all
rustali new blog --force                         # overwrite an existing directory
```

### What each preset generates

Every project gets the same base regardless of preset: a `.provider(DatabaseProvider)`, a
`controllers.rs`, a single `.env` (not `config/*.toml`), and SQLite with
`DATABASE_URL=sqlite://db/app.db?mode=rwc`.

On top of that base:

- **`api`** adds the `openapi` package: a Swagger router mounted with `.routes(openapi_router())`,
  and a controller that serves a text `/` plus a JSON `/api/ping`. No view layer.
- **`fullstack`** adds the `auth` package: Askama views with `templates/home.html`, a users
  model and `create_users` migration, and session wiring.
- **`minimal`** adds nothing beyond the base — just the ORM and one controller.

The generated `main.rs` builds in a fixed order: `DatabaseProvider` → (`SessionProvider`,
`AuthProvider`) if `auth` → cache/mail/queue providers → `.migrations::<Migrator>()` if `auth`
→ `.controllers()` → `.routes(openapi_router())` if `openapi` → `.configure(session_layers)`
if `auth` → `.run().await`.

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

To trim features, turn off the defaults with `default-features = false` and list only what
you need. A JSON API on Postgres with no views, for example:

```toml
[dependencies]
rustali = { version = "0.1", default-features = false, features = ["macros", "orm", "postgres", "auth", "validation"] }
```

A few things to keep in mind when trimming:

- A database driver is separate from `orm`: keep `orm` **and** one of `sqlite` / `postgres` /
  `mysql`. Turning on `orm` without a driver gives you the ORM types but nothing to connect
  to.
- `auth` implies `sessions`, so you get cookie sessions and CSRF even in an API-leaning build
  — drop `auth` too if you truly want neither.
- `redis` and `tera` are opt-in extensions of `cache` and `views`; they do nothing unless the
  base feature is also on.

The `api` preset scaffolds this leaner shape for you: a JSON-only project with the view layer
left out. See [Project type (preset)](#project-type-preset) above.

## Editor setup

Install [rust-analyzer](https://rust-analyzer.github.io/). Point its check command at
clippy for the same lints the framework uses:

```json
// .vscode/settings.json
{ "rust-analyzer.check.command": "clippy" }
```

Next: [Your First App](first-app).
