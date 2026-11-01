//! Caching for the Rustali web framework.
//!
//! A single async [`Cache`] trait with two stores: an in-process [`MemoryCache`]
//! (default, with TTL expiry) and an optional [`RedisCache`] (feature `redis`). The
//! star method is [`CacheExt::remember`] — fetch-or-compute-and-store in one call, the
//! pattern most apps actually want:
//!
//! ```ignore
//! let user = cache.remember("user:42", Duration::from_secs(60), || async {
//!     User::find_or_404(&db, 42).await
//! }).await?;
//! ```
//!
//! Values are serialised with `serde_json`, so any `Serialize + DeserializeOwned` type
//! can be cached. The provider registers a `Cache` as a service; handlers reach it with
//! `Inject<Arc<dyn Cache>>` or `ctx.cache()`.

mod memory;
mod provider;

#[cfg(feature = "redis")]
mod redis_store;

use std::{future::Future, time::Duration};

pub use memory::MemoryCache;
pub use provider::{CacheConfig, CacheContextExt, CacheProvider, SharedCache};
use rustali_core::Result;
use serde::{Serialize, de::DeserializeOwned};

#[cfg(feature = "redis")]
pub use redis_store::RedisCache;

/// An async key/value cache with expiry.
///
/// Implementors store opaque bytes; the typed helpers ([`CacheExt`]) handle
/// serialisation. `ttl = None` means "no expiry".
pub trait Cache: Send + Sync + 'static {
    /// Fetch raw bytes for `key`.
    fn get_bytes<'a>(
        &'a self,
        key: &'a str,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<Option<Vec<u8>>>> + Send + 'a>>;

    /// Store raw bytes under `key` with an optional time-to-live.
    fn put_bytes<'a>(
        &'a self,
        key: &'a str,
        value: Vec<u8>,
        ttl: Option<Duration>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

    /// Remove `key`. Removing a missing key is not an error.
    fn forget<'a>(
        &'a self,
        key: &'a str,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

    /// Remove everything.
    fn flush(&self) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
}

/// Typed, ergonomic methods layered on any [`Cache`]. Blanket-implemented.
pub trait CacheExt: Cache {
    /// Get and deserialize a value.
    fn get<T: DeserializeOwned>(
        &self,
        key: &str,
    ) -> impl Future<Output = Result<Option<T>>> + Send {
        async move {
            match self.get_bytes(key).await? {
                Some(bytes) => Ok(Some(
                    serde_json::from_slice(&bytes).map_err(rustali_core::Error::internal)?,
                )),
                None => Ok(None),
            }
        }
    }

    /// Serialize and store a value with a TTL.
    fn put<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> impl Future<Output = Result<()>> + Send {
        async move {
            let bytes = serde_json::to_vec(value).map_err(rustali_core::Error::internal)?;
            self.put_bytes(key, bytes, Some(ttl)).await
        }
    }

    /// Store a value forever (until evicted or flushed).
    fn forever<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
    ) -> impl Future<Output = Result<()>> + Send {
        async move {
            let bytes = serde_json::to_vec(value).map_err(rustali_core::Error::internal)?;
            self.put_bytes(key, bytes, None).await
        }
    }

    /// Return the cached value, or compute it with `f`, store it for `ttl`, and return
    /// it. The compute closure runs only on a miss.
    fn remember<T, F, Fut>(
        &self,
        key: &str,
        ttl: Duration,
        f: F,
    ) -> impl Future<Output = Result<T>> + Send
    where
        T: Serialize + DeserializeOwned + Send + Sync,
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T>> + Send,
    {
        async move {
            if let Some(hit) = self.get::<T>(key).await? {
                tracing::trace!(key, "cache hit");
                return Ok(hit);
            }
            tracing::trace!(key, "cache miss");
            let value = f().await?;
            self.put(key, &value, ttl).await?;
            Ok(value)
        }
    }

    /// `true` when `key` is present (and unexpired).
    fn has(&self, key: &str) -> impl Future<Output = Result<bool>> + Send {
        async move { Ok(self.get_bytes(key).await?.is_some()) }
    }
}

impl<C: Cache + ?Sized> CacheExt for C {}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    };

    use super::*;

    #[tokio::test]
    async fn remember_computes_once_then_hits() {
        let cache = MemoryCache::new();
        let calls = Arc::new(AtomicU32::new(0));

        for _ in 0..3 {
            let calls = Arc::clone(&calls);
            let value: u64 = cache
                .remember("answer", Duration::from_secs(60), || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(42)
                })
                .await
                .expect("remember");
            assert_eq!(value, 42);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1, "computed exactly once");
        assert!(cache.has("answer").await.unwrap());
        cache.forget("answer").await.unwrap();
        assert!(!cache.has("answer").await.unwrap());
    }
}
