//! Process-global soft-delete switch.
//!
//! Soft delete is a project-wide policy (`database.toml`'s `soft_delete`), not a per-call
//! choice, so the setting is read once at startup by [`DatabaseProvider`] and stored here.
//! `#[derive(Entity)]`-generated `delete` and `query` read it through [`enabled`] to decide
//! whether to stamp `deleted_at` and hide trashed rows — without threading the config
//! through every call site.
//!
//! [`DatabaseProvider`]: crate::DatabaseProvider

use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);

/// Set the global soft-delete policy. Called once by the database provider from config.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

/// Whether soft delete is enabled for this process.
///
/// Generated entity code checks this to choose between a real `DELETE` and stamping
/// `deleted_at`, and whether a default query filters out trashed rows.
#[must_use]
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}
