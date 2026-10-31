//! Layered application configuration.
//!
//! Precedence, lowest to highest: compiled defaults → `config/*.toml` →
//! `config/{APP_ENV}.toml` → `.env` file → process environment variables prefixed
//! with `APP__` (double underscore separates nesting, e.g. `APP__SERVER__PORT=9000`).

use std::{net::IpAddr, path::Path};

use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};

use crate::Result;

/// Root configuration object. Applications extend it with their own sections by
/// deserialising the same sources into their own struct through [`Config::extract`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// General application settings.
    pub app: AppConfig,
    /// HTTP server settings.
    pub server: ServerConfig,
}

/// General application settings (`[app]` table).
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

/// HTTP server settings (`[server]` table).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Interface to bind.
    pub host: IpAddr,
    /// TCP port to bind.
    pub port: u16,
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

impl Config {
    /// Environment variable that selects the profile file (`config/{env}.toml`).
    pub const ENV_VAR: &'static str = "APP_ENV";
    /// Prefix for environment-variable overrides.
    pub const ENV_PREFIX: &'static str = "APP__";

    /// Load configuration from `./config` relative to the current directory.
    pub fn load() -> Result<Self> {
        Self::load_from("config")
    }

    /// Load configuration using `dir` as the directory holding the TOML files.
    pub fn load_from(dir: impl AsRef<Path>) -> Result<Self> {
        Self::figment(dir.as_ref()).extract().map_err(Into::into)
    }

    /// Deserialise an application-defined configuration struct from the same layered
    /// sources. Use this for custom sections instead of re-reading files.
    pub fn extract<T: for<'de> Deserialize<'de>>(dir: impl AsRef<Path>) -> Result<T> {
        Self::figment(dir.as_ref()).extract().map_err(Into::into)
    }

    /// `true` when running under the `production` profile.
    #[must_use]
    pub fn is_production(&self) -> bool {
        self.app.env == "production"
    }

    fn figment(dir: &Path) -> Figment {
        // `.env` is optional and must never override real environment variables.
        let _ = dotenvy::dotenv();
        let profile = std::env::var(Self::ENV_VAR).unwrap_or_else(|_| "development".into());

        Figment::from(Serialized::defaults(Self::default()))
            .merge(Toml::file(dir.join("app.toml")))
            .merge(Toml::file(dir.join("server.toml")))
            .merge(Toml::file(dir.join(format!("{profile}.toml"))))
            .merge(Env::prefixed(Self::ENV_PREFIX).split("__"))
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
}
