# Testing

Rustali ships an in-process test client. It drives your real application — the same
providers, routes and middleware as `main` — without binding a socket, so tests are fast
and deterministic. Error negotiation, sessions and CSRF all behave exactly as in
production.

## Setup

Add the `test` feature in dev-dependencies:

```toml
[dev-dependencies]
rustali = { version = "0.1", features = ["test"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

For tests to build the same app `main` does, expose an `app()` builder from a library
target (see [Project Structure](../getting-started/structure)).

## Writing a test

Build the app, wrap its router in a `TestClient`, and assert on responses:

```rust
use rustali::test::{TestClient, test_config};
use rustali::prelude::*;

#[tokio::test]
async fn home_works() {
    let (_ctx, router) = App::builder()
        .config(test_config())
        .controllers()
        .build()
        .await
        .unwrap();
    let client = TestClient::new(router);

    client.get("/").await
        .assert_ok()
        .assert_see("Welcome");
}
```

`test_config()` gives a test-friendly config: the `test` environment, debug on, and a fixed
app key. It does **not** set a database — add one if your test touches the DB.

## Isolating the database

For a test that hits the database, give each test its own throwaway database so tests never
see each other's rows. A config section is stored in `Config::extra`, so you can point the
`database` section at an in-memory SQLite before building the app:

```rust
fn test_app_config() -> Config {
    let mut config = test_config();
    config.extra.insert(
        "database".into(),
        toml::toml! { url = "sqlite::memory:" }.into(),
    );
    config
}
```

An in-memory SQLite database is created fresh per connection, so each test starts empty. Run
your migrations (or create the schema) at the start of the test, exactly as `main` would.
Point at a throwaway file instead if a test needs the data to persist across connections
within the test.

## The client

A `TestClient` wraps a built `Router` and keeps a **cookie jar** across calls: every request
sends the cookies stored so far and captures any `Set-Cookie` in the response. So a login in
one request is seen by the next — sessions, flash messages and CSRF all work across a
sequence of calls exactly as a browser would drive them. (The client is intentionally not
`Clone`: the jar is shared state scoped to one client.)

| Method | Sends |
|---|---|
| `get(path)` / `delete(path)` | a GET / DELETE |
| `post_json(path, &body)` / `patch_json(path, &body)` | a JSON body |
| `post_form(path, &[(k, v)])` | a URL-encoded form |
| `send(method, path, body, headers)` | an arbitrary request |

Every request method returns a `TestResponse`.

## Assertions

Each `assert_*` returns the response, so they chain:

| Assertion | Checks |
|---|---|
| `assert_ok()` | status is 200 |
| `assert_status(code)` | an exact status |
| `assert_redirect(Some("/to"))` | a 3xx (optionally to a location) |
| `assert_see(text)` / `assert_dont_see(text)` | body contains / omits a substring |
| `assert_json_has(pointer)` | a JSON value exists at an RFC 6901 pointer |
| `assert_json_at(pointer, &value)` | the JSON value equals |
| `assert_header(name, Some(value))` | a header is present / equals |

When you need the raw values instead of an assertion, `TestResponse` exposes them directly:
`text()` (the body as a `String`), `json()` (the body parsed to a `serde_json::Value`) and
`header(name)` (`Option<String>`). The CSRF example below uses `text()` to pull the token
out of the rendered form.

## Example: a form flow

This mirrors the blog's integration test — it drives the whole SSR path, including CSRF and
validation, in process:

```rust
let client = TestClient::new(router);

// the form renders with a CSRF token
let home = client.get("/").await;
home.assert_ok().assert_see("No posts yet.");
let token = extract_token(&home.text());

// an invalid submission redirects back with errors + old input
client.post_form("/posts", &[("_token", &token), ("title", "Hi"), ("body", "")])
    .await
    .assert_redirect(Some("/"));
client.get("/").await.assert_see("between 3 and 200").assert_see("value=\"Hi\"");

// a missing CSRF token is rejected
client.post_form("/posts", &[("title", "x"), ("body", "y")])
    .await
    .assert_status(rustali::http::StatusCode::FORBIDDEN);
```

Because it runs the full stack, one such test exercises routing, the ORM, validation,
sessions and CSRF together.

## Testing authenticated routes

There is no `acting_as` shortcut — and you rarely need one, because the cookie jar lets you
log in the way a user does. Post to the login route, then the session cookie rides along on
the next request to a protected route:

```rust
let client = TestClient::new(router);

// log in (the response's Set-Cookie is stored in the jar)
client
    .post_form("/login", &[("email", "ada@example.com"), ("password", "secret")])
    .await
    .assert_redirect(Some("/dashboard"));

// the next request is authenticated, because the jar replays the session cookie
client.get("/dashboard").await.assert_ok().assert_see("Welcome back");
```

This tests the real login path — password check, session creation, the guard on the
protected route — rather than faking an authenticated state.

> `rustali make:test <name>` scaffolds a test.
