# ADR-0003: Configuration is one TOML file per section, with profile directories

- **Status:** Accepted
- **Date:** 2026-10-31

## Context

The first implementation merged `config/server.toml` at the root of the configuration
tree, so `port = 9000` in that file silently went nowhere (the schema expects
`server.port`). A test caught it. We need a layout that is obvious to Laravel users
(`config/app.php`, `config/database.php`) and cannot fail silently.

## Decision

- `config/<section>.toml` is merged under the key `<section>`. The file name *is* the
  section; files never contain their own `[section]` header.
- `config/<APP_ENV>/<section>.toml` is merged on top for the active profile
  (`APP_ENV`, default `development`).
- `.env` is loaded (never overriding real environment variables), then variables
  prefixed `APP__` override everything (`APP__SERVER__PORT=9000`).
- Files are merged in sorted order for determinism; a missing directory contributes
  nothing; malformed TOML or wrong types are `Error::Config` at startup.
- Unknown sections are preserved so applications can read their own with
  `Config::extract::<T>()`.

## Consequences

- A typo in a file name creates a new section instead of overriding — mitigated by
  the upcoming `rustali config:show` CLI command that prints the merged tree.
- Profile-specific files duplicate keys they override; that is the intended,
  explicit behaviour (same as Laravel's `.env` per environment).
