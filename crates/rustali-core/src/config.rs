//! Layered application configuration.
//!
//! Layout follows Laravel: one file per section.
//!
//! ```text
//! config/
//! ├── app.toml            → [app]      (keys: name, env, debug)
//! ├── server.toml         → [server]   (host, port)
//! ├── log.toml            → [log]
//! ├── database.toml       → [database] (read by rustali-orm)
//! └── production/         → same files, merged on top when APP_ENV=production
//!     └── app.toml
//! ```
//!
//! Precedence, lowest to highest: compiled defaults → `config/<section>.toml` →
//! `config/<APP_ENV>/<section>.toml` → `.env` file → process environment variables
//! prefixed with `APP__` (double underscore separates nesting, e.g.
//! `APP__SERVER__PORT=9000`). Sections the kernel does not know are kept in
//! [`Config::extra`] and read by packages with [`Config::section`].

use std::{
    collections::BTreeMap,
    net::IpAddr,
    path::{Path, PathBuf},
};

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
    /// Enables verbose error pages and hot reload helpers. Never true in production.
    pub debug: bool,
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
    /// Environment variable that selects the profile directory (`config/<env>/`).
    pub const ENV_VAR: &'static str = "APP_ENV";
    /// Prefix for environment-variable overrides.
    pub const ENV_PREFIX: &'static str = "APP__";

    /// Load configuration from `./config` relative to the current directory.
    pub fn load() -> Result<Self> {
        Self::load_from("config")
    }

    /// Load configuration using `dir` as the directory holding the section files.
    pub fn load_from(dir: impl AsRef<Path>) -> Result<Self> {
        Self::figment(dir.as_ref())?.extract().map_err(Into::into)
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

    fn figment(dir: &Path) -> Result<Figment> {
        // `.env` is optional and must never override real environment variables.
        let _ = dotenvy::dotenv();
        let profile = std::env::var(Self::ENV_VAR).unwrap_or_else(|_| "development".into());

        let mut figment = Figment::from(Serialized::defaults(Self::default()));
        figment = Self::merge_dir(figment, dir)?;
        figment = Self::merge_dir(figment, &dir.join(&profile))?;
        Ok(figment.merge(Env::prefixed(Self::ENV_PREFIX).split("__")))
    }

    /// Merge every `<section>.toml` in `dir` under the key `<section>`.
    fn merge_dir(mut figment: Figment, dir: &Path) -> Result<Figment> {
        for path in Self::section_files(dir) {
            let Some(section) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let text = std::fs::read_to_string(&path)?;
            let value: toml::Value = toml::from_str(&text)
                .map_err(|err| Error::bad_config(format!("{}: {err}", path.display())))?;
            figment = figment.merge(Serialized::default(section, value));
        }
        Ok(figment)
    }

    fn section_files(dir: &Path) -> Vec<PathBuf> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new(); // a missing directory simply contributes nothing
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "toml"))
            .collect();
        files.sort(); // deterministic merge order
        files
    }
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
    fn section_files_map_to_sections() {
        figment::Jail::expect_with(|jail| {
            jail.create_dir("config")?;
            jail.create_file("config/server.toml", "port = 9000")?;
            jail.create_file("config/app.toml", "name = \"Shop\"")?;
            let cfg = Config::load_from("config").expect("config loads");
            assert_eq!(cfg.server.port, 9000);
            assert_eq!(cfg.app.name, "Shop");
            Ok(())
        });
    }

    #[test]
    fn profile_directory_overrides_base_files() {
        figment::Jail::expect_with(|jail| {
            jail.create_dir("config/production")?;
            jail.create_file("config/app.toml", "debug = true")?;
            jail.create_file(
                "config/production/app.toml",
                "debug = false\nenv = \"production\"",
            )?;
            jail.set_env("APP_ENV", "production");
            let cfg = Config::load_from("config").expect("config loads");
            assert!(!cfg.app.debug);
            assert!(cfg.is_production());
            Ok(())
        });
    }

    #[test]
    fn env_overrides_win() {
        figment::Jail::expect_with(|jail| {
            jail.create_dir("config")?;
            jail.create_file("config/server.toml", "port = 9000")?;
            jail.set_env("APP__SERVER__PORT", "9100");
            let cfg = Config::load_from("config").expect("config loads");
            assert_eq!(cfg.server.port, 9100);
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
    fn unknown_sections_are_kept_and_typed_on_demand() {
        figment::Jail::expect_with(|jail| {
            jail.create_dir("config")?;
            jail.create_file("config/payments.toml", "provider = \"stripe\"\nretries = 3")?;
            jail.set_env("APP__PAYMENTS__RETRIES", "5");
            let cfg = Config::load_from("config").expect("config loads");
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
    fn section_type_errors_name_the_section() {
        figment::Jail::expect_with(|jail| {
            jail.create_dir("config")?;
            jail.create_file("config/payments.toml", "retries = \"many\"")?;
            let cfg = Config::load_from("config").expect("config loads");
            let err = cfg.section::<Payments>("payments").unwrap_err();
            assert!(err.to_string().contains("[payments]"), "{err}");
            Ok(())
        });
    }

    #[test]
    fn wrong_types_are_errors() {
        figment::Jail::expect_with(|jail| {
            jail.create_dir("config")?;
            jail.create_file("config/server.toml", "port = \"not-a-number\"")?;
            assert!(matches!(Config::load_from("config"), Err(Error::Config(_))));
            Ok(())
        });
    }

    #[test]
    fn malformed_toml_is_an_error() {
        figment::Jail::expect_with(|jail| {
            jail.create_dir("config")?;
            jail.create_file("config/app.toml", "name = ")?;
            assert!(matches!(Config::load_from("config"), Err(Error::Config(_))));
            Ok(())
        });
    }
}
