//! Application configuration from a single `.env` file.
//!
//! All settings live in one `.env` file (or the real process environment) as flat,
//! readable keys — there are no per-section config files:
//!
//! ```text
//! APP_NAME=Shop
//! APP_ENV=production
//! APP_KEY=a-long-secret
//! PORT=8080
//! DATABASE_URL=sqlite://db/app.db?mode=rwc
//! CACHE_DRIVER=redis
//! ```
//!
//! Precedence, lowest to highest: compiled defaults → `.env` file → the real process
//! environment. Each known key maps to a config field (see [`ENV_KEYS`]); keys for optional
//! packages populate [`Config::extra`], which packages read with [`Config::section`]. For
//! settings without a clean key, an `APP__SECTION__FIELD` escape hatch is still honoured
//! (double underscore separates nesting).

use std::{collections::BTreeMap, net::IpAddr};

use figment::{
    Figment,
    providers::{Env, Serialized},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{Error, Result};

/// Root configuration object.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// General application settings.
    pub app: AppConfig,
    /// HTTP server settings.
    pub server: ServerConfig,
    /// Logging settings.
    pub log: LogConfig,
    /// Every other section, keyed by name (`database`, `mail`, your own...).
    #[serde(flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

/// General application settings (`config/app.toml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Human-readable application name, used in logs and default page titles.
    pub name: String,
    /// Deployment environment: `development`, `test` or `production`.
    pub env: String,
    /// Enables verbose error detail in responses. Turn it off for production (set
    /// `debug = false` in `config/production/app.toml`); it is not forced off automatically.
    pub debug: bool,
    /// Secret used to sign cookies and tokens (`APP__APP__KEY`, usually set from `.env`).
    /// Must be at least 16 characters. Empty in development means "generate a throwaway key
    /// at boot".
    pub key: String,
}

/// HTTP server settings (`config/server.toml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Interface to bind.
    pub host: IpAddr,
    /// TCP port to bind.
    pub port: u16,
}

/// Logging settings (`config/log.toml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LogConfig {
    /// `tracing` filter directive, e.g. `info` or `info,tower_http=debug`.
    /// Overridden by the `RUST_LOG` environment variable when present.
    pub level: String,
    /// Emit one JSON object per line instead of human-readable text.
    pub json: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "Rustali".into(),
            env: "development".into(),
            debug: true,
            key: String::new(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: IpAddr::from([127, 0, 0, 1]),
            port: 8000,
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info,tower_http=debug".into(),
            json: false,
        }
    }
}

impl Config {
    /// Prefix for the `APP__SECTION__FIELD` escape-hatch overrides.
    pub const ENV_PREFIX: &'static str = "APP__";

    /// Load configuration from `.env` (if present) and the process environment.
    pub fn load() -> Result<Self> {
        Self::figment().extract().map_err(Into::into)
    }

    /// Deserialise a package- or application-defined section, e.g.
    /// `config.section::<DatabaseConfig>("database")`. `Ok(None)` when absent.
    pub fn section<T: DeserializeOwned>(&self, name: &str) -> Result<Option<T>> {
        self.extra.get(name).map_or_else(
            || Ok(None),
            |value| {
                value
                    .clone()
                    .try_into::<T>()
                    .map(Some)
                    .map_err(|err| Error::bad_config(format!("[{name}] {err}")))
            },
        )
    }

    /// Like [`Self::section`] but falls back to `T::default()` when absent.
    pub fn section_or_default<T: DeserializeOwned + Default>(&self, name: &str) -> Result<T> {
        self.section(name).map(Option::unwrap_or_default)
    }

    /// `true` when running under the `production` profile.
    #[must_use]
    pub fn is_production(&self) -> bool {
        self.app.env == "production"
    }

    fn figment() -> Figment {
        // `.env` is optional and must never override real environment variables.
        let _ = dotenvy::dotenv();

        let mut figment = Figment::from(Serialized::defaults(Self::default()));
        // Map each known clean key (e.g. DATABASE_URL) to its dotted config path.
        for (key, path) in ENV_KEYS {
            if let Ok(value) = std::env::var(key) {
                figment = figment.merge(Serialized::default(path, parse_scalar(&value)));
            }
        }
        // Escape hatch: APP__SECTION__FIELD for settings without a clean key.
        figment.merge(Env::prefixed(Self::ENV_PREFIX).split("__"))
    }
}

/// Map from a clean `.env` key to the dotted config path it sets.
///
/// Keys for `app` / `server` / `log` fill the typed top-level sections; the rest populate
/// [`Config::extra`] under their package's section name, where providers read them.
pub const ENV_KEYS: &[(&str, &str)] = &[
    // app
    ("APP_NAME", "app.name"),
    ("APP_ENV", "app.env"),
    ("APP_DEBUG", "app.debug"),
    ("APP_KEY", "app.key"),
    // server
    ("HOST", "server.host"),
    ("PORT", "server.port"),
    // log
    ("LOG_LEVEL", "log.level"),
    ("LOG_JSON", "log.json"),
    // database (rustali-orm)
    ("DATABASE_URL", "database.url"),
    ("DATABASE_MAX_CONNECTIONS", "database.max_connections"),
    ("DATABASE_LOGGING", "database.logging"),
    ("SOFT_DELETE", "database.soft_delete"),
    // cache
    ("CACHE_DRIVER", "cache.driver"),
    ("CACHE_URL", "cache.url"),
    // mail
    ("MAIL_TRANSPORT", "mail.transport"),
    ("MAIL_FROM", "mail.from"),
    ("MAIL_URL", "mail.url"),
    // queue
    ("QUEUE_CAPACITY", "queue.capacity"),
    // session
    ("SESSION_COOKIE", "session.cookie"),
    ("SESSION_LIFETIME_MINUTES", "session.lifetime_minutes"),
    ("SESSION_SAME_SITE", "session.same_site"),
    ("SESSION_CSRF", "session.csrf"),
    // openapi
    ("OPENAPI_TITLE", "openapi.title"),
    ("OPENAPI_VERSION", "openapi.version"),
];

/// Parse a `.env` string value into the most specific TOML scalar (bool, integer, else
/// string), so typed fields like `PORT` / `APP_DEBUG` deserialise correctly.
fn parse_scalar(value: &str) -> toml::Value {
    value
        .parse::<bool>()
        .map(toml::Value::Boolean)
        .or_else(|_| value.parse::<i64>().map(toml::Value::Integer))
        .unwrap_or_else(|_| toml::Value::String(value.to_owned()))
}

impl Error {
    /// Configuration failure with a custom message (unreadable or malformed file).
    pub(crate) fn bad_config(message: impl Into<String>) -> Self {
        Self::Config(Box::new(figment::Error::from(message.into())))
    }
}

#[cfg(test)]
// Tests may panic on failure; figment's `Jail` closure also returns its large error.
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::result_large_err)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_development_friendly() {
        let cfg = Config::default();
        assert_eq!(cfg.server.port, 8000);
        assert!(cfg.app.debug);
        assert!(!cfg.is_production());
        assert!(cfg.extra.is_empty());
    }

    #[test]
    fn clean_keys_map_to_fields() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("APP_NAME", "Shop");
            jail.set_env("PORT", "9000");
            jail.set_env("APP_DEBUG", "false");
            let cfg = Config::load().expect("config loads");
            assert_eq!(cfg.app.name, "Shop");
            assert_eq!(cfg.server.port, 9000); // parsed as an integer
            assert!(!cfg.app.debug); // parsed as a bool
            Ok(())
        });
    }

    #[test]
    fn env_selects_production() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("APP_ENV", "production");
            let cfg = Config::load().expect("config loads");
            assert!(cfg.is_production());
            Ok(())
        });
    }

    #[test]
    fn package_keys_populate_sections() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("DATABASE_URL", "sqlite://x.db");
            jail.set_env("CACHE_DRIVER", "redis");
            let cfg = Config::load().expect("config loads");
            assert_eq!(cfg.extra["database"]["url"].as_str(), Some("sqlite://x.db"));
            assert_eq!(cfg.extra["cache"]["driver"].as_str(), Some("redis"));
            Ok(())
        });
    }

    #[derive(Debug, Default, Deserialize, PartialEq)]
    #[serde(default)]
    struct Payments {
        provider: String,
        retries: u32,
    }

    #[test]
    fn escape_hatch_fills_custom_sections() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("APP__PAYMENTS__PROVIDER", "stripe");
            jail.set_env("APP__PAYMENTS__RETRIES", "5");
            let cfg = Config::load().expect("config loads");
            let payments: Payments = cfg.section("payments").expect("typed").expect("present");
            assert_eq!(
                payments,
                Payments {
                    provider: "stripe".into(),
                    retries: 5
                }
            );
            assert!(cfg.section::<Payments>("missing").expect("ok").is_none());
            assert_eq!(
                cfg.section_or_default::<Payments>("missing").expect("ok"),
                Payments::default()
            );
            Ok(())
        });
    }

    #[test]
    fn wrong_types_are_errors() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("PORT", "not-a-number");
            assert!(matches!(Config::load(), Err(Error::Config(_))));
            Ok(())
        });
    }
}
