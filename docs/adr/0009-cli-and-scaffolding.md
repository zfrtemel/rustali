# ADR-0009: The `rustali` CLI — scaffolding and generators

- **Status:** Accepted
- **Date:** 2026-11-01

## Context

VISION §4.10 / the "kur, çalıştır, işine başla" promise: `cargo install rustali-cli` →
`rustali new blog` → `cd blog && cargo run` gives a working app in minutes. The CLI also
provides Laravel-style `make:*` generators. App-level commands (serve, migrate,
migrate:diff, routes, queue:work) already live *in the app binary* (ADR-0005/0007), not
the CLI.

## Decision

1. **Two responsibilities only:** `rustali new <name>` scaffolds a project, and
   `rustali make:<thing> <Name>` generates a source file. Everything else is the app's
   own command, run with `cargo run -- <command>`. The CLI says so when given an unknown
   command rather than trying to proxy cargo.
2. **Colon convention, hand-parsed.** `make:controller` matches Laravel. clap's derive
   does not model colons in subcommand names cleanly, so `main` inspects `argv` directly:
   `new`, `help`, `--version`, and `make:*` (split on the colon). This keeps the familiar
   surface without contorting the arg parser.
3. **Templates are embedded string constants** rendered with minijinja. The CLI is a
   single self-contained binary with no runtime template files to ship or find.
4. **Generators are project-aware.** `make:*` walks up to the project root (a directory
   with `Cargo.toml` + `src/`) and writes under `src/` (controllers/, models/,
   migrations/, jobs/, ...). Files are never overwritten without `--force`.
5. **`new` scaffolds the convention:** `Cargo.toml` (depends on `rustali` + the `test`
   feature in dev-deps), `config/{app,database}.toml`, `.env`, `src/main.rs`,
   `src/controllers.rs` (a home controller with an SSR route and a JSON `/api/ping`), a
   `templates/home.html` (omitted with `--api`), and `db/`. The generated `main.rs` is
   the 8-line `App::builder().provider(DatabaseProvider).controllers().run()`.

## Consequences

- Verified end to end: `rustali new demo` → (point deps at the local workspace, since
  nothing is published yet) → `cargo check` passes → the binary boots and serves. The
  `make:controller`/`make:model` generators write the expected files into a project.
- **Closes the ADR-0007 gap for new code:** scaffolded projects keep migrations under
  `src/migrations/` and `make:migration` writes there; `migrate:diff` still writes to
  `db/migrations/` (the generated file is moved into `src/` and listed in the Migrator).
  Fully automatic placement+wiring of a diffed migration remains a later polish.
- Until crates are published, generated `Cargo.toml` references `rustali = "0.1"`, which
  only resolves once 0.1.0 is on crates.io (Faz 8 release step). Local testing patches the
  path. Acceptable for pre-alpha.
- The CLI does not yet have a `doctor` command or `--vite` asset pipeline (VISION
  mentioned both); deferred.
