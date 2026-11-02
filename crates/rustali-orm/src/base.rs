//! [`BaseEntity`]: the common shape every scaffolded model shares.
//!
//! Rust has no class inheritance, so a shared "base model" is expressed as a trait of
//! accessors rather than a base struct. A model that carries the conventional `id`,
//! `created_at` and `updated_at` fields implements [`BaseEntity`] (the `make:model`
//! scaffold writes the impl for you), and generic code can then work over "any model with
//! these fields" — logging, auditing, serialisation helpers — without naming the concrete
//! type.
//!
//! A shared base model in Rust form: common *behaviour* through a trait, with the fields still
//! declared on each struct (and managed by `#[derive(Entity)]`'s timestamp and key
//! conventions).

use chrono::{DateTime, Utc};

/// Accessors shared by every model with the conventional `id` / `created_at` /
/// `updated_at` fields.
///
/// Implement it by hand, or let `rustali make:model` write the impl. For soft-delete
/// support, also implement [`SoftDelete`].
///
/// ```ignore
/// impl BaseEntity for Post {
///     type Id = i64;
///     fn id(&self) -> Self::Id { self.id }
///     fn created_at(&self) -> DateTime<Utc> { self.created_at }
///     fn updated_at(&self) -> DateTime<Utc> { self.updated_at }
/// }
/// ```
pub trait BaseEntity {
    /// The primary-key type (usually `i64`).
    type Id: Copy + Send + Sync;

    /// The primary key.
    fn id(&self) -> Self::Id;

    /// When the row was created.
    fn created_at(&self) -> DateTime<Utc>;

    /// When the row was last updated.
    fn updated_at(&self) -> DateTime<Utc>;
}

/// A model that supports soft deletion: it carries a nullable `deleted_at` instead of being
/// removed from the table.
///
/// Soft delete is opt-in per project (see the database config). A model implements this when
/// its scaffold includes the `deleted_at` field; the accessor lets generic code tell a live
/// row (`deleted_at() == None`) from a trashed one.
pub trait SoftDelete: BaseEntity {
    /// When the row was soft-deleted, or `None` while it is live.
    fn deleted_at(&self) -> Option<DateTime<Utc>>;

    /// `true` when the row has been soft-deleted.
    fn is_trashed(&self) -> bool {
        self.deleted_at().is_some()
    }
}
