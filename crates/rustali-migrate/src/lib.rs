//! Database migrations for the Rustali web framework.
//!
//! Migrations are ordinary [`sea_orm_migration`] migrations compiled into the
//! application. This crate adds the glue: a type-erased [`Runner`] registered as a
//! service, the `migrate*` console commands, and the [`MigrateAppExt::migrations`]
//! builder method.
//!
//! ```ignore
//! App::builder()
//!     .provider(DatabaseProvider)
//!     .migrations::<db::migrations::Migrator>()
//!     .run()
//!     .await
//! ```
//!
//! Then: `cargo run -- migrate`, `migrate:rollback --step 1`, `migrate:fresh`,
//! `migrate:status`, `migrate:reset`.

pub mod commands;
pub mod runner;

pub use runner::{MigrateAppExt, Migrations, Runner};
pub use sea_orm_migration;

/// What a migration file imports.
///
/// Besides SeaORM's migration prelude this exports the crate names `sea_orm_migration`
/// and `sea_orm`, because `DeriveMigrationName` / `DeriveIden` expand to paths starting
/// with them and an application normally depends only on `rustali`.
pub mod prelude {
    pub use sea_orm_migration;
    pub use sea_orm_migration::{prelude::*, sea_orm};
}
