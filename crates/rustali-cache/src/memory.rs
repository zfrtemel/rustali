//! In-process cache with lazy TTL expiry.

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Mutex, PoisonError},
    time::{Duration, Instant},
};

use rustali_core::Result;

use crate::Cache;

struct Entry {
    value: Vec<u8>,
    expires_at: Option<Instant>,
}

/// A thread-safe in-memory cache.
///
/// Entries expire lazily (on access) and during an occasional sweep, so no background
/// task is needed. Good for a single process; use [`crate::RedisCache`] to share across
/// instances.
#[derive(Default)]
pub struct MemoryCache {
    entries: Mutex<HashMap<String, Entry>>,
}

impl MemoryCache {
    /// An empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Entry>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Cache for MemoryCache {
    fn get_bytes<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Vec<u8>>>> + Send + 'a>> {
        Box::pin(async move {
            let now = Instant::now();
            let mut entries = self.lock();
            let result = match entries.get(key) {
                Some(entry) if entry.expires_at.is_none_or(|exp| exp > now) => {
                    Some(entry.value.clone())
                }
                Some(_) => {
                    entries.remove(key); // expired
                    None
                }
                None => None,
            };
            drop(entries);
            Ok(result)
        })
    }

    fn put_bytes<'a>(
        &'a self,
        key: &'a str,
        value: Vec<u8>,
        ttl: Option<Duration>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let now = Instant::now();
            let mut entries = self.lock();
            // Opportunistic sweep so dropped-TTL entries do not accumulate unbounded.
            if entries.len() > 1024 {
                entries.retain(|_, e| e.expires_at.is_none_or(|exp| exp > now));
            }
            entries.insert(
                key.to_owned(),
                Entry {
                    value,
                    expires_at: ttl.map(|d| now + d),
                },
            );
            drop(entries);
            Ok(())
        })
    }

    fn forget<'a>(&'a self, key: &'a str) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            self.lock().remove(key);
            Ok(())
        })
    }

    fn flush(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            self.lock().clear();
            Ok(())
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::CacheExt;

    #[tokio::test]
    async fn stores_and_expires() {
        let cache = MemoryCache::new();
        cache
            .put("k", &"v".to_owned(), Duration::from_millis(30))
            .await
            .unwrap();
        assert_eq!(
            cache.get::<String>("k").await.unwrap().as_deref(),
            Some("v")
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(cache.get::<String>("k").await.unwrap(), None, "expired");
    }

    #[tokio::test]
    async fn forever_does_not_expire() {
        let cache = MemoryCache::new();
        cache.forever("k", &7u32).await.unwrap();
        assert_eq!(cache.get::<u32>("k").await.unwrap(), Some(7));
        cache.flush().await.unwrap();
        assert_eq!(cache.get::<u32>("k").await.unwrap(), None);
    }
}
