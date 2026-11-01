# ADR-0008: Ecosystem packages — cache, mail, queue, test kit

- **Status:** Accepted
- **Date:** 2026-11-01

## Context

VISION §4.9 lists the "batteries" beyond the web/data core: cache, mail, queue (and
later storage, events, i18n). Each should be a tak-çıkar package registered through the
`Provider` mechanism (ADR-0004/0005/0006), with an in-process default that works with
zero infrastructure and a production backend behind the same trait.

## Decision

Three crates, each: one async trait, a memory/log default, an optional heavy backend, a
`Provider`, a config section, and `ctx.xxx()` sugar.

1. **`rustali-cache`** — `Cache` (async get/put/forget/flush over bytes) with a typed
   `CacheExt` layer (`get`/`put`/`forever`/`remember`/`has`, serde_json). `MemoryCache`
   (default, lazy TTL expiry) and `RedisCache` (feature `redis`). `remember(key, ttl, f)`
   is the headline: compute-on-miss-and-store.
2. **`rustali-mail`** — `Mailable` (subject + HTML/text) → `Email` → `Transport`.
   `LogTransport` (default; writes to tracing), `MemoryTransport` (tests), `SmtpTransport`
   (feature `smtp`, lettre + rustls/ring/webpki-roots). `Mailer` holds a transport and a
   default `from`.
3. **`rustali-queue`** — `Job` (async `handle(&AppContext)`, `NAME`, `RETRY`) dispatched
   through the `Queue` trait; `MemoryQueue` is a bounded channel drained by a `Worker`
   task; `queue:work` command runs it. Retries use fixed backoff. Jobs serialise to JSON
   and route by `NAME` through a handler registry built with `Worker::builder().register::<J>()`.

4. **`rustali-test`** — an in-process `TestClient` (via `tower::ServiceExt::oneshot`,
   no socket) that keeps cookies across calls, plus a `TestResponse` with fluent
   assertions (`assert_ok`, `assert_redirect`, `assert_see`, `assert_json_at`, ...). It
   runs the full middleware stack, so error negotiation, sessions and CSRF behave exactly
   as in production. Gated behind a `test` feature (apps put it in `dev-dependencies`).
   Proven by the `blog` integration test, which drives the SSR validation/flash/CSRF flow
   end to end.

All register a shared service (except the test kit) and are reachable from the prelude;
the meta crate gates them behind default-on features (`cache`, `mail`, `queue`; `test`
and `redis` opt-in).

## Consequences

- An app adds `.provider(CacheProvider)` / `.provider(MailProvider)` /
  `.provider(QueueProvider::new(worker))` and uses `ctx.cache()`, `ctx.mailer()`,
  `ctx.queue()`. The in-process defaults need no Redis/SMTP to develop against.
- **Durability gap (documented):** `MemoryQueue` loses jobs on restart; a durable
  database/Redis backend plugs in behind `Queue` later without touching job code. This is
  why `apalis` was *not* adopted yet — its version churn and heavier surface weren't worth
  it before the trait boundary proved out. The plan named apalis; we chose a lean
  self-contained worker that apalis (or sqlx) can back behind `Queue`.
- A scheduler (cron), storage (`opendal`), events and i18n remain for a later pass; the
  same provider+trait shape applies.
- The queue `Worker` is built by the app (it must name its job types for deserialisation
  routing); there is no inventory auto-collection here because a job's generic `handle`
  cannot be registered without naming the type.
