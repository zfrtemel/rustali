# ADR-0003: All configuration from a single `.env` file

- **Status:** Accepted (supersedes the earlier "one TOML file per section" design)
- **Date:** 2026-11-02

## Context

The first design used one TOML file per section (`config/app.toml`, `config/database.toml`, …)
merged by file name, plus profile directories and `APP__`-prefixed env overrides. It worked but
spread settings across many files and made a typo in a file name silently create a new section.
We want one obvious place for all settings, and flat, readable keys.

## Decision

- **One `.env` file** (or the real process environment) is the single source of configuration.
  There are no `config/*.toml` files and no profile directories.
- Keys are **flat and clean** — `APP_NAME`, `APP_ENV`, `APP_KEY`, `PORT`, `DATABASE_URL`,
  `CACHE_DRIVER`, … — each mapped to a config field by an explicit table (`config::ENV_KEYS`).
  Keys for optional packages populate the typed section a provider reads (`DATABASE_URL` →
  the `database` section, etc.).
- Precedence, lowest to highest: compiled defaults → `.env` → real process environment.
- An `APP__SECTION__FIELD` escape hatch (double underscore = nesting) remains for custom or
  nested settings that have no clean key.
- Values parse to the most specific scalar (bool, integer, else string); a wrong type is
  `Error::Config` at startup.

## Consequences

- One file to read and edit; secrets come from the real environment in production, overriding
  the committed dev `.env`.
- Adding a new clean key means one line in the `ENV_KEYS` table; until then the `APP__` escape
  hatch covers it.
- Flat keys cannot express arbitrary nested structures directly — the escape hatch is there for
  the rare case that needs it.
