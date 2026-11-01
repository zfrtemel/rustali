# Cache

The cache is one async trait with two stores — an in-process memory store (default) and an
optional Redis store. The method you will reach for most is `remember`:
fetch-or-compute-and-store in one call.

## Setup

Add `CacheProvider`; configure in `config/cache.toml`:

```rust
App::builder().provider(CacheProvider).controllers().run().await
```

```toml
# config/cache.toml
driver = "memory"                 # memory | redis
url = "redis://127.0.0.1:6379"    # used when driver = "redis"
```

The `redis` driver needs the `redis` Cargo feature.

## Using it

Reach the cache with `ctx.cache()` (or `Inject<SharedCache>`). Values are serialised with
JSON, so any `Serialize + DeserializeOwned` type works.

```rust
use std::time::Duration;

#[get("/stats")]
async fn stats(ctx: Ctx, db: Db) -> Result<Json<Stats>> {
    let cache = ctx.cache()?;
    let stats = cache
        .remember("stats", Duration::from_secs(60), || async {
            compute_stats(&db).await           // only runs on a cache miss
        })
        .await?;
    Ok(Json(stats))
}
```

### Methods

| Method | Does |
|---|---|
| `remember(key, ttl, f)` | Return the cached value, or compute with `f`, store for `ttl`, return it. |
| `get::<T>(key)` | `Option<T>`. |
| `put(key, &value, ttl)` | Store with an expiry. |
| `forever(key, &value)` | Store with no expiry. |
| `has(key)` | `bool`. |
| `forget(key)` | Remove a key. |
| `flush()` | Remove everything. |

```rust
let cache = ctx.cache()?;
cache.put("user:42", &user, Duration::from_secs(300)).await?;
let hit: Option<User> = cache.get("user:42").await?;
cache.forget("user:42").await?;
```

## Stores

- **Memory** (default) — a process-local map with lazy TTL expiry. No infrastructure, great
  for development and single-node apps. Not shared across instances.
- **Redis** (`redis` feature, `driver = "redis"`) — shared across instances, survives
  restarts.

Both sit behind the same `Cache` trait, so switching stores is a config change; your code
does not move.
