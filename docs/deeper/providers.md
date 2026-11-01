# Service Container & Providers

Rustali has an explicit, type-indexed service container. Services are registered by **type**
at startup and looked up by type at request time. There is no string binding and no
auto-wiring magic.

## Registering a service

The simplest way is `provide` — hand the builder a ready value:

```rust
#[derive(Clone)]
struct Clock { /* ... */ }

App::builder()
    .provide(Clock::new())
    .controllers()
```

## Injecting a service

Ask for it in a handler with `Inject<T>`, which dereferences to `T`:

```rust
#[get("/now")]
async fn now(clock: Inject<Clock>) -> String {
    clock.iso8601()        // Inject<Clock> derefs to Clock
}
```

Or reach it through the context anywhere you have one:

```rust
let clock = ctx.service::<Clock>()?;     // Arc<Clock>, or Err if not registered
```

A service that nobody registered yields a clear error naming the type, so a forgotten
provider fails loudly rather than silently.

### Services are shared

Every registered value is wrapped in an `Arc` and handed out by shared reference:
`Inject<T>` holds an `Arc<T>` and derefs to `T`; `ctx.service::<T>()` returns the `Arc<T>`
directly. So a service is **registered once and shared** by every request — there is one
`Clock`, one database pool, one HTTP client, not a copy per request. Two consequences:

- A service must be `Send + Sync + 'static` (it is touched from many tasks at once).
- It should be cheap to share, not mutate in place. For interior mutability use the usual
  tools (`Mutex`, `RwLock`, atomics, or a connection pool that manages its own locking).

Registration is **by type**: one value per type. Registering a second value of the same type
replaces the first. If you need two of the same underlying type, wrap each in a distinct
newtype.

## Providers

A **provider** is a package's startup hook. It reads its own configuration section and
registers what it owns. The framework's own
subsystems are providers (`DatabaseProvider`, `SessionProvider`, `CacheProvider`,
`MailProvider`, `AuthProvider`, `QueueProvider`), and you can write your own.

```rust
use rustali::prelude::*;

struct PaymentsProvider;

impl Provider for PaymentsProvider {
    fn name(&self) -> &'static str { "payments" }

    fn register<'a>(&'a self, registry: &'a mut Registry<'_>) -> RegisterFuture<'a> {
        Box::pin(async move {
            // read config/payments.toml, build a client, register it
            let config: PaymentsConfig = registry.config().section_or_default("payments")?;
            registry.insert(PaymentsClient::connect(&config).await?);
            Ok(())
        })
    }
}

App::builder().provider(PaymentsProvider).controllers().run().await
```

### Lifecycle and ordering

Providers run **in the order added**, after configuration loads and before routes are built.
That ordering is a contract you can rely on: a later provider can depend on an earlier one's
services by pulling them out of the registry with `registry.require::<T>()`:

```rust
fn register<'a>(&'a self, registry: &'a mut Registry<'_>) -> RegisterFuture<'a> {
    Box::pin(async move {
        let db = registry.require::<Database>()?;   // registered by DatabaseProvider earlier
        registry.insert(InvoiceStore::new(db));
        Ok(())
    })
}
```

`require` errors if the dependency is missing, so an out-of-order registration is a clear
startup failure, not a confusing `None` later. `register` is async because setup often means
connecting — a database pool, a cache, a mail transport.

### Config section

The convention is one TOML section per provider, read with `section_or_default`:

```rust
let config: PaymentsConfig = registry.config().section_or_default("payments")?;
```

`section_or_default` falls back to `PaymentsConfig::default()` when `config/payments.toml`
(or the `[payments]` section) is absent, so a provider works out of the box and is
configured only when needed. The config type is an ordinary
`#[derive(Deserialize, Default)]` struct.

### Registering commands

A provider can also register console commands with `registry.command(...)` — that is how
`migrate` and `queue:work` get added by their providers. See [Console Commands](console).

## Request-scoped data

The container holds shared, long-lived services. For per-request values, use request
extensions through middleware, or just pass data down from the handler — the container is
for things like pools and clients, not per-request state.

## Why explicit

The trade-off is deliberate: you write one line to register and one type to inject, and in
exchange there is no runtime reflection, no magic resolution order, and the compiler checks
every service type. A missing dependency is an error with a name, not a mystery at runtime.
