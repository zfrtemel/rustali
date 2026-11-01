//! Config, the registered service handle, and the cache provider.

use std::sync::Arc;

use rustali_core::{AppContext, Provider, RegisterFuture, Registry, Result};
use serde::{Deserialize, Serialize};

use crate::{Cache, MemoryCache};

/// Shared handle stored as a service.
pub type SharedCache = Arc<dyn Cache>;

/// `config/cache.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheConfig {
    /// Backend: `memory` (default) or `redis` (requires the `redis` feature).
    pub driver: String,
    /// Redis connection URL, used when `driver = "redis"`.
    pub url: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            driver: "memory".into(),
            url: "redis://127.0.0.1:6379".into(),
        }
    }
}

impl CacheConfig {
    /// Section name.
    pub const SECTION: &'static str = "cache";
}

/// Registers a [`Cache`] from `config/cache.toml`.
#[derive(Debug, Default, Clone, Copy)]
pub struct CacheProvider;

impl Provider for CacheProvider {
    fn name(&self) -> &'static str {
        "cache"
    }

    fn register<'a>(&'a self, registry: &'a mut Registry<'_>) -> RegisterFuture<'a> {
        Box::pin(async move {
            let config: CacheConfig = registry.config().section_or_default(CacheConfig::SECTION)?;
            let cache = build(&config)?;
            registry.insert(cache);
            Ok(())
        })
    }
}

#[cfg(not(feature = "redis"))]
fn build(config: &CacheConfig) -> Result<SharedCache> {
    if config.driver == "redis" {
        return Err(rustali_core::Error::bad_request(
            "cache driver `redis` needs the `redis` feature on rustali-cache",
        ));
    }
    Ok(Arc::new(MemoryCache::new()))
}

#[cfg(feature = "redis")]
fn build(config: &CacheConfig) -> Result<SharedCache> {
    match config.driver.as_str() {
        "redis" => Ok(Arc::new(crate::RedisCache::connect(&config.url)?)),
        _ => Ok(Arc::new(MemoryCache::new())),
    }
}

/// `ctx.cache()` sugar.
pub trait CacheContextExt {
    /// The registered cache, or an error when [`CacheProvider`] was not added.
    fn cache(&self) -> Result<SharedCache>;
}

impl CacheContextExt for AppContext {
    fn cache(&self) -> Result<SharedCache> {
        self.service::<SharedCache>().map(|arc| (*arc).clone())
    }
}
