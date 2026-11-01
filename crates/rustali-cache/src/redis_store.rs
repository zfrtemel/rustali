//! Redis-backed cache (feature `redis`).

use std::{future::Future, pin::Pin, time::Duration};

use redis::AsyncCommands;
use rustali_core::{Error, Result};

use crate::Cache;

/// A cache backed by a Redis server, shared across application instances.
pub struct RedisCache {
    client: redis::Client,
}

impl RedisCache {
    /// Connect (lazily; the client dials on first use) to `url`.
    pub fn connect(url: &str) -> Result<Self> {
        let client = redis::Client::open(url).map_err(Error::internal)?;
        Ok(Self { client })
    }

    async fn conn(&self) -> Result<redis::aio::MultiplexedConnection> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(Error::internal)
    }
}

impl Cache for RedisCache {
    fn get_bytes<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Vec<u8>>>> + Send + 'a>> {
        Box::pin(async move {
            let mut conn = self.conn().await?;
            let value: Option<Vec<u8>> = conn.get(key).await.map_err(Error::internal)?;
            Ok(value)
        })
    }

    fn put_bytes<'a>(
        &'a self,
        key: &'a str,
        value: Vec<u8>,
        ttl: Option<Duration>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut conn = self.conn().await?;
            match ttl {
                Some(ttl) => conn
                    .set_ex::<_, _, ()>(key, value, ttl.as_secs().max(1))
                    .await
                    .map_err(Error::internal),
                None => conn
                    .set::<_, _, ()>(key, value)
                    .await
                    .map_err(Error::internal),
            }
        })
    }

    fn forget<'a>(&'a self, key: &'a str) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut conn = self.conn().await?;
            conn.del::<_, ()>(key).await.map_err(Error::internal)
        })
    }

    fn flush(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            let mut conn = self.conn().await?;
            redis::cmd("FLUSHDB")
                .query_async::<()>(&mut conn)
                .await
                .map_err(Error::internal)
        })
    }
}
