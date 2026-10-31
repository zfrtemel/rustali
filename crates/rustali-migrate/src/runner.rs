//! Type-erased access to a [`MigratorTrait`] implementation.

use std::{future::Future, marker::PhantomData, pin::Pin};

use rustali_core::{AppBuilder, Error, Result};
use rustali_orm::sea_orm::DatabaseConnection;
use sea_orm_migration::MigratorTrait;

/// Boxed future used by [`Runner`].
pub type RunFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// Status of one migration file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationStatus {
    /// File/module name, e.g. `m20261031_000001_create_users`.
    pub name: String,
    /// Already applied to the database.
    pub applied: bool,
}

/// Object-safe view over a [`MigratorTrait`]; registered as a service so console
/// commands and the CLI can drive migrations without knowing the concrete type.
pub trait Runner: Send + Sync + 'static {
    /// Apply pending migrations (`steps = None` for all).
    fn up<'a>(&'a self, db: &'a DatabaseConnection, steps: Option<u32>) -> RunFuture<'a, ()>;
    /// Roll back applied migrations (`steps = None` for all).
    fn down<'a>(&'a self, db: &'a DatabaseConnection, steps: Option<u32>) -> RunFuture<'a, ()>;
    /// Drop every table, then apply all migrations.
    fn fresh<'a>(&'a self, db: &'a DatabaseConnection) -> RunFuture<'a, ()>;
    /// Roll back everything, then apply everything.
    fn refresh<'a>(&'a self, db: &'a DatabaseConnection) -> RunFuture<'a, ()>;
    /// Status of every known migration.
    fn status<'a>(&'a self, db: &'a DatabaseConnection) -> RunFuture<'a, Vec<MigrationStatus>>;
}

/// [`Runner`] implementation for any `M: MigratorTrait`.
pub struct Migrations<M>(PhantomData<fn() -> M>);

impl<M> Default for Migrations<M> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<M> Migrations<M> {
    /// Wrap the migrator type.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<M: MigratorTrait + 'static> Runner for Migrations<M> {
    fn up<'a>(&'a self, db: &'a DatabaseConnection, steps: Option<u32>) -> RunFuture<'a, ()> {
        Box::pin(async move { M::up(db, steps).await.map_err(Error::database) })
    }

    fn down<'a>(&'a self, db: &'a DatabaseConnection, steps: Option<u32>) -> RunFuture<'a, ()> {
        Box::pin(async move { M::down(db, steps).await.map_err(Error::database) })
    }

    fn fresh<'a>(&'a self, db: &'a DatabaseConnection) -> RunFuture<'a, ()> {
        Box::pin(async move { M::fresh(db).await.map_err(Error::database) })
    }

    fn refresh<'a>(&'a self, db: &'a DatabaseConnection) -> RunFuture<'a, ()> {
        Box::pin(async move { M::refresh(db).await.map_err(Error::database) })
    }

    fn status<'a>(&'a self, db: &'a DatabaseConnection) -> RunFuture<'a, Vec<MigrationStatus>> {
        Box::pin(async move {
            let migrations = M::get_migration_with_status(db)
                .await
                .map_err(Error::database)?;
            Ok(migrations
                .into_iter()
                .map(|m| MigrationStatus {
                    name: m.name().to_owned(),
                    applied: matches!(m.status(), sea_orm_migration::MigrationStatus::Applied),
                })
                .collect())
        })
    }
}

/// Service wrapper stored in the application context.
pub struct MigrationRunner(pub Box<dyn Runner>);

/// `App::builder().migrations::<Migrator>()`.
pub trait MigrateAppExt {
    /// Register the application's migrator and the `migrate*` commands.
    #[must_use]
    fn migrations<M: MigratorTrait + 'static>(self) -> Self;
}

impl MigrateAppExt for AppBuilder {
    fn migrations<M: MigratorTrait + 'static>(self) -> Self {
        self.provide(MigrationRunner(Box::new(Migrations::<M>::new())))
            .command(crate::commands::Migrate)
            .command(crate::commands::Rollback)
            .command(crate::commands::Fresh)
            .command(crate::commands::Refresh)
            .command(crate::commands::Reset)
            .command(crate::commands::Status)
    }
}
