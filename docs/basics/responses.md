# Responses & Errors

## Returning a response

Any handler returns a type that implements `IntoResponse`. The common ones:

| Return type | Becomes |
|---|---|
| `&'static str`, `String` | `text/plain` |
| `Json<T>` | `application/json` |
| `Html<String>`, `View<T>` | `text/html` |
| `Redirect::to("/path")` | a 303 redirect |
| `StatusCode` | an empty response with that status |
| `(StatusCode, T)` | `T`’s response with the status overridden |
| `Result<T>` | `T` on `Ok`, a rendered error on `Err` |

```rust
#[post("/")]
async fn store(db: Db, Json(input): Json<CreateTodo>) -> Result<(StatusCode, Json<Todo>)> {
    let todo = Todo::new(input.title.trim()).insert(&db).await?;
    Ok((StatusCode::CREATED, Json(todo)))
}
```

## Errors are values

There are no exceptions. Fallible work returns `Result<T>` — the alias for
`Result<T, rustali::Error>` — and `?` propagates failures. The framework turns an `Error`
into the right HTTP response for you.

```rust
#[get("/{id}")]
async fn show(db: Db, Path(id): Path<i64>) -> Result<Json<Todo>> {
    // find_or_404 returns Err(Error::NotFound) when missing; `?` turns it into a 404
    Ok(Json(Todo::find_or_404(&db, id).await?))
}
```

### The error type

`rustali::Error` maps each variant to an HTTP status:

| Variant | Status | Constructor |
|---|---|---|
| `NotFound` | 404 | `Error::NotFound` |
| `BadRequest(msg)` | 400 | `Error::bad_request("...")` |
| `Unauthorized` | 401 | `Error::Unauthorized` |
| `Forbidden` | 403 | `Error::Forbidden` |
| `Validation(errors)` | 422 | from the [validated](validation) extractors |
| `TooManyRequests { .. }` | 429 | from `Throttle` |
| `Database(..)` | 500 | `Error::database(e)` (ORM maps `DbErr` for you) |
| `Config(..)` | 500 | configuration load failure |
| `Internal(..)` | 500 | `Error::internal(e)` — any other error |

`?` converts common errors automatically: `std::io::Error` and `figment::Error` have
`From` impls, and the ORM maps SeaORM's `DbErr` (a missing row becomes `NotFound`, the
rest `Database`). Only `BadRequest` and `Validation` expose their detail to clients;
`Internal`/`Database` details are shown **only when `app.debug` is on**, so production
never leaks internals.

## Content negotiation

The same error renders differently depending on who is asking — this is what lets one
controller serve SSR and an API. The rule:

- If `Accept` contains `text/html` → an **HTML error page**.
- Otherwise, if `Accept` mentions `json`, **or** the path starts with `/api/` → **JSON**.

A browser hitting a missing page gets a styled 404 page; a `fetch()` to `/api/...` gets an
RFC 9457 *Problem Details* document:

```json
{
  "type": "about:blank",
  "title": "Not Found",
  "status": 404,
  "instance": "/api/todos/999",
  "request_id": "a1b2c3d4-..."
}
```

Validation failures add an `errors` object; client-safe errors add `detail`. Every
response also carries an `x-request-id` header, echoed into the JSON body, to correlate
logs.

## Custom error pages

Override the rendering with `App::builder().on_error(...)` — see
[Middleware & Filters](middleware#overriding-error-rendering). It receives the error and
some request facts and may return its own response or defer to the default.

## Panics

A panic in a handler is caught and turned into a `500` (the default stack's catch-panic
layer), so one bad request never takes down the server. A panic is still a bug — fix it;
the catch is a safety net, not error handling.
