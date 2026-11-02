# ADR-0001: Single Cargo workspace with lockstep-versioned crates

- **Status:** Accepted
- **Date:** 2026-10-31
- **Deciders:** project founder, Claude (implementation partner)

## Context

Rustali is a batteries-included web framework. Users expect to add **one**
dependency and get HTTP, ORM, migrations, views, auth, queue, mail, cache and a CLI
that all agree on versions. Internally, however, the pieces must stay separable so
that (a) compile times stay bounded via feature flags, (b) a user can depend on only
`rustali-core` + `rustali-orm` for a microservice, and (c) third parties can publish
packages against stable sub-crate APIs.

A single development monorepo split into independently publishable sub-crates is the common
answer; SeaORM and tokio follow the same pattern in Rust.

## Decision

1. One git repository, one Cargo workspace. All framework crates live under
   `crates/`, starters under `starters/`, examples under `examples/`.
2. Every crate is independently publishable on crates.io, but **all crates share a
   single version** (`workspace.package.version`) and are released together.
3. All third-party dependency versions are declared once in
   `[workspace.dependencies]`; crates use `dep = { workspace = true }`.
4. Dependency direction is acyclic and enforced by review:
   `core ← router ← orm ← migrate`, `core ← {view, session, auth, validate, queue,
   mail, cache, storage, events, i18n, test}`, everything ← `cli`. The proc-macro
   crate depends only on `syn`/`quote`/`proc-macro2` and emits paths under
   `rustali::__private`.
5. The meta crate `rustali` re-exports everything behind feature flags and provides
   `rustali::prelude::*`. It is the only crate end users are expected to name.

## Consequences

- **Positive:** one `cargo update` story for users; no version-skew bugs between
  sub-crates; a single CI pipeline; simple release automation (`cargo-release`).
- **Negative:** a patch release in one crate bumps all crates; the workspace
  `Cargo.lock` grows large; contributors must understand the layering rule.
- **Mitigation:** `xtask ci` checks the dependency direction; `cargo-deny` and
  `cargo-udeps` keep the graph lean.

## Alternatives considered

- *Separate repositories per crate* — rejected: version skew, duplicated CI, worse
  contributor experience.
- *Independent versions per crate (like tokio)* — rejected for now: more flexible but
  confusing for a framework whose selling point is "everything fits together".
  Revisit at 1.0 if sub-crates stabilise at different rates.
