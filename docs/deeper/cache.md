# Cache

The cache is one async trait with two stores — an in-process memory store (default) and an
optional Redis store. The method you will reach for most is `remember`:
fetch-or-compute-and-store in one call.

## Two traits, one cache

The cache is split into two traits, and knowing which is which explains the whole API:

- **`Cache`** is the low-level store contract: it moves **opaque bytes** (`get_bytes`,
  `put_bytes`, `forget`, `flush`). Each backend — memory, Redis — implements this.
- **`CacheExt`** is the typed, ergonomic layer **blanket-implemented** on every `Cache`. It
  adds `get::<T>`, `put`, `forever`, `remember` and `has`, handling JSON
  serialisation so you work with your own types, not bytes.

You almost always call `CacheExt` methods. The split means a new backend only has to move
bytes correctly; every typed helper comes for free.

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

The `redis` driver needs the `redis` Cargo feature. Without it, asking for `driver =
"redis"` is a clear startup error rather than a silent fallback, so a misconfigured
production deploy fails fast instead of quietly caching in-process.

## Using it

Reach the cache with `ctx.cache()`, which returns a `SharedCache` (`Arc<dyn Cache>`) you can
clone and move into tasks. Values are serialised with JSON, so any
`Serialize + DeserializeOwned` type works.

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

## remember in depth

`remember` is the workhorse. Its closure runs **only on a miss**, and it must return a
`Result<T>` — the same `Result` your handlers use — so the `?` inside it propagates errors
straight out of `remember`:

```rust
let user = cache
    .remember(&format!("user:{id}"), Duration::from_secs(300), || async {
        // runs only when the key is absent or expired
        User::find_or_404(&db, id).await       // -> Result<User>
    })
    .await?;
```

A few things worth knowing:

- The closure is `FnOnce` and returns a future, so the `|| async { ... }` shape is what you
  want — not `async || ...`.
- If the compute step fails, nothing is cached and the error is returned; the next call
  tries again.
- The stored type must be `Serialize + DeserializeOwned` (it round-trips through JSON).

## Invalidation

There is no automatic invalidation — you decide when a cached value is stale. The common
pattern is to `forget` a key right after the write that makes it outdated:

```rust
// after updating the user, drop the cached copy
user.update(&db).await?;
cache.forget(&format!("user:{id}")).await?;
```

For values that should refresh on a fixed cadence, prefer a short TTL on `remember` over
manual invalidation — simpler, and self-healing.

## Stores

- **Memory** (default) — a process-local map. Entries expire **lazily** (checked on access)
  and during an occasional sweep, so there is no background thread to manage. No
  infrastructure, great for development and single-node apps. It is **not shared** across
  instances, and it is cleared when the process restarts.
- **Redis** (`redis` feature, `driver = "redis"`) — shared across every instance and
  survives restarts. Use it once you run more than one process, or want the cache to outlive
  a deploy.

Both sit behind the same `Cache` trait, so switching stores is a config change; your code
does not move. Because the memory store is per-process, a value cached on one node is a miss
on another — fine for derived data behind `remember`, but do not rely on it for anything
that must be consistent across instances (use Redis for that).

## When to cache

The cache is for **derived, re-computable** data: expensive aggregates, rendered fragments,
results of a slow upstream call. It is not a database — anything you cannot recompute should
live in a real store. The mental model for `remember` is "a fast path in front of a source
of truth", never the source of truth itself.
