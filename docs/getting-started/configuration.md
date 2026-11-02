# Configuration

All configuration lives in a single **`.env`** file (or the real process environment), as
flat, readable keys. There are no per-section config files — one place for everything.

```bash
# .env
APP_NAME=My App
APP_ENV=development        # development | test | production
APP_DEBUG=true            # verbose error detail in responses — turn off for production
APP_KEY=a-long-dev-secret-change-me

DATABASE_URL=sqlite://db/app.db?mode=rwc
```

## Precedence

From lowest to highest, each layer overrides the one before:

1. Compiled defaults (every field has one — e.g. the port is `8000`).
2. The `.env` file.
3. Real process environment variables.

So setting `PORT` in your deploy platform's environment overrides the `PORT` in `.env`, which
overrides the built-in default. A `.env` file is a developer convenience; production values
come from the real environment and win.

## The keys

Each setting is a clean, flat key. The common ones:

| Key | Sets |
|---|---|
| `APP_NAME` | application name (logs, default page titles) |
| `APP_ENV` | `development` \| `test` \| `production` |
| `APP_DEBUG` | verbose error detail in responses (turn off for production) |
| `APP_KEY` | secret that signs cookies and JWTs (≥ 16 chars) |
| `HOST` / `PORT` | the address to bind (`127.0.0.1` / `8000`) |
| `LOG_LEVEL` / `LOG_JSON` | tracing filter; one JSON object per line when `true` |
| `DATABASE_URL` | database connection string |
| `DATABASE_MAX_CONNECTIONS` / `DATABASE_LOGGING` | pool size; log every statement |
| `SOFT_DELETE` | stamp `deleted_at` instead of deleting ([details](../database/base-entity)) |
| `CACHE_DRIVER` / `CACHE_URL` | cache backend ([details](../deeper/cache)) |
| `MAIL_TRANSPORT` / `MAIL_FROM` / `MAIL_URL` | mail backend ([details](../deeper/mail)) |
| `QUEUE_CAPACITY` | in-process queue size ([details](../deeper/queues)) |
| `SESSION_COOKIE` / `SESSION_LIFETIME_MINUTES` / `SESSION_CSRF` | sessions ([details](../security/csrf)) |
| `OPENAPI_TITLE` / `OPENAPI_VERSION` | OpenAPI document info |

Values parse to the most specific type — `PORT=8000` is an integer, `APP_DEBUG=false` a bool —
so a wrong type (e.g. `PORT=abc`) is a clear error at startup, not a silent default.

## Per-environment settings

There are no profile files; set per-environment values in each environment itself. Commit a
dev-friendly `.env`, and in production set the real values (secrets, `APP_ENV=production`,
`APP_DEBUG=false`) through the deploy platform's environment, where they override `.env`.

`APP_DEBUG` is **not** forced off automatically in production — set `APP_DEBUG=false` there.
With debug on, error responses include internal detail (stack messages, database errors) you
do not want to expose to real users.

## The key

`APP_KEY` signs session cookies and JWTs and must be at least 16 characters. In development the
scaffold ships a throwaway key; in production set a real secret from the environment and never
commit it:

```bash
# your deploy platform's environment (not the committed .env)
APP_KEY=a-long-random-secret-at-least-32-characters
```

## Reading configuration

In a handler, reach configuration through the context:

```rust
async fn home(ctx: Ctx) -> String {
    ctx.config().app.name.clone()
}
```

`ctx.config().is_production()` is a convenience for `APP_ENV == "production"`.

## Custom settings

Package and application settings beyond the built-in keys are read as typed sections with
`Config::section`. For a custom section, set its values with the `APP__SECTION__FIELD` escape
hatch (double underscore separates nesting):

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Payments {
    provider: String,
    retries: u32,
}

// .env:  APP__PAYMENTS__PROVIDER=stripe   APP__PAYMENTS__RETRIES=3
let payments: Option<Payments> = ctx.config().section("payments")?;
```

Use `section_or_default::<T>("name")` when the type implements `Default` and the values may be
absent.

> A package that needs configuration (database, cache, mail, …) reads its section this way
> inside its [provider](../deeper/providers), so there is one consistent mechanism for
> framework and application config alike.
