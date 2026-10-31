//! Procedural macros for the Rustali web framework.
//!
//! This crate is an implementation detail re-exported by `rustali`; it must only
//! depend on the proc-macro toolkit and emit paths under `::rustali::__private`.
//!
//! Macros arrive with their phases: `#[controller]`/`#[get]` (Faz 3),
//! `#[derive(Entity)]` (Faz 2), `#[derive(Validate)]` (Faz 4), `#[authorize]` and
//! `#[middleware]` (Faz 3).
