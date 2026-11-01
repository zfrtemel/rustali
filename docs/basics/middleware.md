# Middleware & Filters

Middleware wraps a request on its way in and the response on its way out. In Rustali
middleware is just a [tower] `Layer`, and you attach it at three levels — globally, per
controller, or per route — so cross-cutting concerns (rate limiting, CORS, auth,
logging) stay out of your handlers. This is the layered filter pipeline from the
framework's design.

[tower]: https://docs.rs/tower

## The default stack

Every application is wrapped in a default stack, outermost first:

1. **Request ID** — assigns an `x-request-id` (a UUID) and propagates it.
2. **Tracing** — structured request/response logging via `tracing`.
3. **Catch-panic** — a panicking handler becomes a `500` instead of dropping the
   connection.
4. **Context injection** — puts the `AppContext` in request extensions so plain tower
   layers can reach services.
5. **Error negotiation** — turns a framework [`Error`](responses) into JSON or an HTML
   page depending on the client (the "exception filter").
6. **Your layers** — everything you add below.
7. **The routes.**

You never add these yourself; they are always on.

## Global middleware

Add a layer around every route with `App::builder().layer(...)`:

```rust
use rustali::{middleware::Cors, prelude::*};

App::builder()
    .layer(Cors::permissive())
    .controllers()
    .run()
    .await
```

Your global layers sit *inside* the default stack, so they still see the request id and
their errors are still rendered consistently.

### CORS

`Cors` is `tower_http`'s `CorsLayer`, re-exported. `Cors::permissive()` allows any origin,
method and header — convenient in development, but for production you usually want to name
what you allow:

```rust
use rustali::http::{HeaderValue, Method};
use rustali::middleware::Cors;

let cors = Cors::new()
    .allow_origin("https://app.example.com".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST]);

App::builder().layer(cors)
```

You only need CORS when a browser calls your API from a different origin; a same-origin SSR
app does not.

## Controller and route middleware

Attach a layer to a whole controller with `#[middleware(expr)]` on the `impl`, or to one
route with `#[middleware(expr)]` on the method. Method layers run inside controller
layers.

```rust
#[controller("/api")]
#[middleware(Throttle::per_minute(600))]   // every route in this controller
impl ApiController {
    #[get("/feed")]
    async fn feed() -> Result<Json<Vec<Post>>> { /* ... */ }

    #[post("/import")]
    #[middleware(Throttle::per_minute(5))]  // stricter, just this route
    async fn import() -> Result<StatusCode> { /* ... */ }
}
```

The `expr` is any value whose type is a tower `Layer` — a framework layer like
`Throttle`, an axum helper like `axum::middleware::from_fn(my_fn)`, or your own.

## Rate limiting

`Throttle` is a fixed-window, in-memory limiter. It adds an `x-ratelimit-remaining`
header and answers `429 Too Many Requests` with `Retry-After` when the limit is hit.

```rust
Throttle::per_minute(60)             // 60 requests / minute
Throttle::per_second(10)             // 10 / second
Throttle::new(100, Duration::from_secs(300))   // 100 per 5 minutes
```

Clients are keyed by `X-Forwarded-For` (first address) or the peer IP. This is the
single-process implementation; a shared store (Redis) will plug in behind it.

## Writing your own

Any axum middleware function works. Wrap it with `from_fn` and add it at whatever level
fits:

```rust
use rustali::http::{extract::Request, middleware::{from_fn, Next}, response::Response};

async fn timing(request: Request, next: Next) -> Response {
    let start = std::time::Instant::now();
    let response = next.run(request).await;
    tracing::info!(elapsed = ?start.elapsed(), "handled");
    response
}

// global:
App::builder().layer(from_fn(timing))
```

`rustali make:middleware Timing` scaffolds this shape for you.

### Short-circuiting

A middleware does not have to call `next.run(...)`. Return a response before it and the
handler never runs — this is how a gate, a maintenance switch or a header check rejects a
request early:

```rust
use rustali::http::{
    extract::Request, middleware::Next, response::{IntoResponse, Response}, StatusCode,
};

async fn require_api_key(request: Request, next: Next) -> Response {
    match request.headers().get("x-api-key") {
        Some(key) if key == "expected" => next.run(request).await,
        _ => StatusCode::UNAUTHORIZED.into_response(),   // handler never runs
    }
}
```

The two things a middleware can do — act on the request before `next.run`, and act on the
response after — cover the whole surface: authentication, timing, injecting headers, rewriting
responses, and refusing a request outright.

### How the levels nest

The three attachment points nest from outside in: **global → controller → route → handler**.
An outer layer sees the request first and the response last. So a global timing layer
measures everything inside it, a controller `Throttle` counts before a route's own stricter
`Throttle`, and a route `#[authorize]` runs closest to the handler. Reach for the narrowest
level that does the job: global for truly cross-cutting concerns, controller for a shared
policy, route for a one-off.

## Authorization as a filter

`#[authorize("ability")]` (or `#[authorize(role = "admin")]`) is a filter that runs
before the handler, checks the current user against the application's `Gate`, and answers
`401` (not logged in) or `403` (denied). It is covered in
[Authorization](../security/authorization).

```rust
#[post("/posts")]
#[authorize(role = "editor")]
async fn store(/* ... */) -> Result<Redirect> { /* ... */ }
```

## Overriding error rendering

The error-negotiation layer is the exception filter; replace its output with
`App::builder().on_error(...)`. The closure receives the error and some request facts and
returns `Some(response)` to take over, or `None` to fall back to the default JSON/HTML:

```rust
use rustali::http::{response::{IntoResponse, Redirect}, StatusCode};

App::builder().on_error(|error, request| {
    // Send browsers to a friendly 404 page, leave API callers to the JSON default.
    (error.status == StatusCode::NOT_FOUND && !request.wants_json)
        .then(|| Redirect::to("/not-found").into_response())
})
```

`error` is an `ErrorBody` (`status`, `title`, an optional public `detail`, and a `debug`
flag); `request` is an `ErrorRequest` (`path`, `wants_json`, `request_id`, `debug`). Use
`wants_json` to branch on the client — the same signal the default negotiation uses.

## Layers that need config

Some layers (sessions, CSRF) need values from `config/` that only exist after the app is
assembled. Add those in `configure`, which runs with the context available:

```rust
App::builder()
    .provider(SessionProvider)
    .configure(session_layers)   // adds the cookie-session + CSRF layers
    .controllers()
    .run()
    .await
```

See [CSRF & Sessions](../security/csrf).
