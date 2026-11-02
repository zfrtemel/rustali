# ADR-0006: Authentication — storage-agnostic users, a guard chain, argon2id

- **Status:** Accepted
- **Date:** 2026-11-01

## Context

VISION §4.7 wants auth that "ships ready to use": register/login/logout working after
`rustali new`, password reset, and both cookie (SSR) and token (API) sessions, wired
into the `#[authorize]`/`Gate` built in Faz 3 (ADR-0005). The kernel already defines
`Actor`, `ActorResolver`, `Policy` and `Gate`; auth must supply the concrete pieces
without the kernel depending on a database or a password library.

## Decision

1. **`Authenticatable`** is the application's user contract: `auth_id()`,
   `password_hash()`, `roles()`. It is storage-agnostic (usually a `#[derive(Entity)]`
   struct). `AuthUser<U>` adapts it to the kernel's `Actor`, so policies and
   `role:<name>` work unchanged.
2. **`UserStore`** is implemented by the application to load users by id (for
   session/token resolution) and by login field (for credential checks). It is
   type-erased behind a service so the resolver and guards share it.
3. **Guard chain.** `AuthProvider::new(store)` registers an `ActorResolver` that tries
   guards in order (default `[Session, Bearer]`); the first that yields an id whose
   user loads wins. `Session` reads the id from the cookie session; `Bearer` verifies a
   JWT. The chain is configurable with `.guards([...])`.
4. **Passwords: argon2id** through `password::hash`/`verify`. Hashes are PHC strings;
   there is no API returning plaintext, and verification is constant-time. Parameters
   can change without a migration.
5. **JWT (feature `jwt`, default on):** HS256 signed with `app.key` (≥16 chars
   required even in dev); `Claims { sub, exp, iat }`, 5s leeway. `jwt::issue`/`verify`.
6. **Handler extractors:** `Auth<U>` (401 when absent), `OptionalAuth<U>`, `Guest<U>`
   (403 when logged in), and `Login<U>` for `attempt`/`login`/`logout`, all over the
   request session. Session login calls `cycle_id()` to prevent fixation.
7. **No `unsafe`.** Extractors use axum's `async fn from_request_parts(&mut Parts, ..)`
   directly; an earlier attempt to move `Parts` out and back (needed `unsafe`) was
   abandoned — the straightforward signature borrows `parts` for the whole future.
8. **Layering:** `rustali-auth` depends on `rustali-core` and `rustali-session`
   (for the cookie session), nothing lower reaches up. The `auth` meta-feature implies
   `sessions`.

## Consequences

- An app implements two small traits and adds three providers; register/login/logout
  and a protected page work, proven by the `auth-app` example over HTTP (argon2
  hashing, 303 redirects, 401 gating, session id rotation).
- Password reset and email verification (mail-dependent) and the personal-access-token
  API guard are deferred to Faz 7 (they need the mail and cache packages).
- `app.key` is now load-bearing for JWT and signed cookies; `rustali key:generate`
  (Faz 7 CLI) will produce it, and the starter ships a dev key with a warning.
