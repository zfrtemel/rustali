# Configuration

Configuration lives in the `config/` directory as TOML, **one file per section**. Values
are layered so you can keep defaults in files and override per environment or from the
environment.

## Layout

```text
config/
├── app.toml          # [app]      name, env, debug, key
├── server.toml       # [server]   host, port
├── log.toml          # [log]      level, json
├── database.toml     # [database] url, pool settings   (read by the ORM)
└── production/       # overrides merged when APP_ENV=production
    └── app.toml
```

The file name **is** the section — `server.toml` populates the `server` section; the file
does not repeat a `[server]` header.

## Precedence

From lowest to highest, each layer overrides the one before:

1. Compiled defaults (every field has one — e.g. `server.port` is `8000`).
2. `config/<section>.toml`.
3. `config/<APP_ENV>/<section>.toml` — the active profile (`APP_ENV`, default
   `development`).
4. A `.env` file (optional; never overrides real environment variables).
5. Process environment variables prefixed `APP__`, with `__` separating nesting.

So `APP__SERVER__PORT=9000` sets `server.port`, and a value in
`config/production/app.toml` beats the one in `config/app.toml` when `APP_ENV=production`.

### Per-environment overrides

The profile directory is how you change behaviour between environments without editing the
base files. For example, keep `debug = true` in `config/app.toml` for development, and turn it
off (plus any production-only setting) in the profile:

```toml
# config/production/app.toml  — merged only when APP_ENV=production
debug = false
```

`debug` is **not** forced off automatically in production; set it here. With `debug` on, error
responses include internal detail (stack messages, database errors), which you do not want to
expose to real users — so turning it off is part of going to production.

### Files vs environment variables

Use `config/` files for non-secret defaults you commit, and environment variables (or `.env`
locally) for secrets and per-deploy values. A real environment variable always wins over
`.env`, so a `.env` file is a developer convenience that never silently overrides what your
deploy platform sets.

## The built-in sections

```toml
# config/app.toml
name = "My App"       # shown in logs and default page titles
env = "development"   # development | test | production
debug = true          # verbose error detail in responses — turn off for production
key = "..."           # ≥16-char secret: signs cookies and JWTs
```

```toml
# config/server.toml
host = "127.0.0.1"
port = 8000
```

```toml
# config/log.toml
level = "info,tower_http=debug"   # a tracing filter; RUST_LOG overrides it
json = false                       # one JSON object per line when true
```

`database.toml` is documented with the [ORM](../database/entities); packages like cache,
mail, queue and sessions each read their own section (`cache`, `mail`, `queue`,
`session`).

## The `key`

`app.key` signs session cookies and JWTs and must be at least 16 characters. In
development the scaffold ships a throwaway key; in production set a real secret from the
environment and never commit it:

```bash
# .env (git-ignored) or your deploy environment
APP__APP__KEY=a-long-random-secret-at-least-32-characters
```

## Reading configuration

In a handler, reach configuration through the context:

```rust
async fn home(ctx: Ctx) -> String {
    ctx.config().app.name.clone()
}
```

`ctx.config().is_production()` is a convenience for `app.env == "production"`.

## Custom sections

Add your own section file and deserialise it into your own type. Define a struct, then
read it with `Config::section`:

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Payments {
    provider: String,
    retries: u32,
}

// config/payments.toml  ->  provider = "stripe"\n  retries = 3
let payments: Option<Payments> = ctx.config().section("payments")?;
```

Use `section_or_default::<T>("name")` when the type implements `Default` and the file may
be absent. Environment overrides work for custom sections too
(`APP__PAYMENTS__RETRIES=5`).

> A package that needs configuration (database, cache, mail, ...) reads its section this
> way inside its [provider](../deeper/providers), so there is one consistent mechanism
> for framework and application config alike.
