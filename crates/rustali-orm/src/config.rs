//! `config/database.toml`.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Database connection settings (`config/database.toml`).
///
/// ```toml
/// url = "sqlite://db/app.db?mode=rwc"
/// max_connections = 10
/// logging = false
/// ```
///
/// The URL is usually supplied through the environment instead:
/// `APP__DATABASE__URL=postgres://user:pass@localhost/app`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    /// Connection URL understood by SeaORM / `SQLx`.
    pub url: String,
    /// Upper bound of the connection pool.
    pub max_connections: u32,
    /// Connections kept open while idle.
    pub min_connections: u32,
    /// Seconds to wait for a connection before failing.
    pub connect_timeout_secs: u64,
    /// Seconds an idle connection may live before being closed.
    pub idle_timeout_secs: u64,
    /// Log every SQL statement at `debug` level.
    pub logging: bool,
    /// Treat `deleted_at` as a soft-delete marker across models that have it: `delete`
    /// stamps the column instead of removing the row, and queries hide trashed rows by
    /// default. Off by default; see [`crate::soft_delete`].
    pub soft_delete: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://db/app.db?mode=rwc".into(),
            max_connections: 10,
            min_connections: 1,
            connect_timeout_secs: 8,
            idle_timeout_secs: 600,
            logging: false,
            soft_delete: false,
        }
    }
}

impl DatabaseConfig {
    /// Section name inside the configuration tree.
    pub const SECTION: &'static str = "database";

    /// Translate into SeaORM connect options.
    #[must_use]
    pub fn connect_options(&self) -> sea_orm::ConnectOptions {
        let mut options = sea_orm::ConnectOptions::new(self.url.clone());
        options
            .max_connections(self.max_connections)
            .min_connections(self.min_connections)
            .connect_timeout(Duration::from_secs(self.connect_timeout_secs))
            .idle_timeout(Duration::from_secs(self.idle_timeout_secs))
            .sqlx_logging(self.logging)
            .sqlx_logging_level(tracing::log::LevelFilter::Debug);
        options
    }

    /// `true` for `sqlite:` URLs.
    #[must_use]
    pub fn is_sqlite(&self) -> bool {
        self.url.starts_with("sqlite:")
    }
}
