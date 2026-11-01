//! Connecting the pool and registering it as a service.

use std::ops::Deref;

use rustali_core::{AppContext, Error, Provider, RegisterFuture, Registry, Result};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, ExecResult, QueryResult, Statement,
    prelude::async_trait::async_trait,
};

use crate::DatabaseConfig;

/// The connection pool as a registered service.
///
/// Dereferences to [`DatabaseConnection`], so `&*db` (or the [`crate::Db`]
/// extractor) can be handed to any SeaORM API.
#[derive(Debug, Clone)]
pub struct Database {
    conn: DatabaseConnection,
    config: DatabaseConfig,
}

impl Database {
    /// Connect using `config`.
    pub async fn connect(config: DatabaseConfig) -> Result<Self> {
        if config.url.trim().is_empty() {
            return Err(Error::bad_request(
                "database.url is empty; set it in config/database.toml or APP__DATABASE__URL",
            ));
        }
        tracing::debug!(url = %redact(&config.url), "connecting to database");
        let conn = sea_orm::Database::connect(config.connect_options())
            .await
            .map_err(Error::database)?;
        Ok(Self { conn, config })
    }

    /// Wrap an existing connection (tests, custom pools).
    #[must_use]
    pub const fn from_connection(conn: DatabaseConnection, config: DatabaseConfig) -> Self {
        Self { conn, config }
    }

    /// The underlying SeaORM connection.
    #[must_use]
    pub const fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }

    /// The settings this pool was created from.
    #[must_use]
    pub const fn config(&self) -> &DatabaseConfig {
        &self.config
    }
}

impl Deref for Database {
    type Target = DatabaseConnection;

    fn deref(&self) -> &Self::Target {
        &self.conn
    }
}

#[async_trait]
impl ConnectionTrait for Database {
    fn get_database_backend(&self) -> DbBackend {
        self.conn.get_database_backend()
    }

    async fn execute_raw(&self, stmt: Statement) -> Result<ExecResult, DbErr> {
        self.conn.execute_raw(stmt).await
    }

    async fn execute_unprepared(&self, sql: &str) -> Result<ExecResult, DbErr> {
        self.conn.execute_unprepared(sql).await
    }

    async fn query_one_raw(&self, stmt: Statement) -> Result<Option<QueryResult>, DbErr> {
        self.conn.query_one_raw(stmt).await
    }

    async fn query_all_raw(&self, stmt: Statement) -> Result<Vec<QueryResult>, DbErr> {
        self.conn.query_all_raw(stmt).await
    }

    fn support_returning(&self) -> bool {
        self.conn.support_returning()
    }
}

/// Reads `config/database.toml`, connects and registers [`Database`].
///
/// ```ignore
/// App::builder().provider(DatabaseProvider).serve().await
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct DatabaseProvider;

impl Provider for DatabaseProvider {
    fn name(&self) -> &'static str {
        "database"
    }

    fn register<'a>(&'a self, registry: &'a mut Registry<'_>) -> RegisterFuture<'a> {
        Box::pin(async move {
            let config: DatabaseConfig = registry
                .config()
                .section_or_default(DatabaseConfig::SECTION)?;
            crate::soft_delete::set_enabled(config.soft_delete);
            let database = Database::connect(config).await?;
            registry.insert(database);
            Ok(())
        })
    }
}

/// `ctx.db()` sugar on the application context.
pub trait DbExt {
    /// The registered [`Database`], or a descriptive error when
    /// [`DatabaseProvider`] was not added.
    fn db(&self) -> Result<std::sync::Arc<Database>>;
}

impl DbExt for AppContext {
    fn db(&self) -> Result<std::sync::Arc<Database>> {
        self.service::<Database>()
    }
}

/// Hide credentials when logging a connection URL.
fn redact(url: &str) -> String {
    match (url.find("://"), url.rfind('@')) {
        (Some(scheme_end), Some(at)) if at > scheme_end => {
            format!("{}://***@{}", &url[..scheme_end], &url[at + 1..])
        }
        _ => url.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_hides_credentials_only() {
        assert_eq!(redact("postgres://u:p@host/db"), "postgres://***@host/db");
        assert_eq!(redact("sqlite://db/app.db"), "sqlite://db/app.db");
    }
}
