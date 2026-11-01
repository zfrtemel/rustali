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

`test_config()` gives a test-friendly config (test env, debug on, a fixed app key); override
sections as needed — e.g. point the database at a throwaway SQLite file.

## The client

A `TestClient` keeps cookies across calls, so a login in one request is seen by the next.

| Method | Sends |
|---|---|
| `get(path)` / `delete(path)` | a GET / DELETE |
| `post_json(path, &body)` / `patch_json(path, &body)` | a JSON body |
| `post_form(path, &[(k, v)])` | a URL-encoded form |
| `send(method, path, body, headers)` | an arbitrary request |

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

> `rustali make:test <name>` scaffolds a test.
