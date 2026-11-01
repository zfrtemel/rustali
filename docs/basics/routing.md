# Routing & Controllers

In Rustali a route lives **on the handler that serves it**. You do not maintain a
separate route file; you annotate a method with the HTTP verb and path, and the
`#[controller]` macro wires it into the router. This is the .NET-style attribute routing
from the framework's design — the route and the code that answers it sit together, and
`cargo run -- routes` prints the whole table (Laravel's `route:list`, back again).

## A first controller

```rust
use rustali::prelude::*;

pub struct HomeController;

#[controller("/")]
impl HomeController {
    #[get("/", name = "home")]
    async fn home() -> &'static str {
        "Welcome to Rustali!"
    }
}
```

Three things to notice:

- `#[controller("/")]` goes on an **inherent `impl`** and declares a path prefix for
  every route inside.
- `#[get("/")]` declares a route. Its path is **joined** with the controller prefix, so
  this handler answers `GET /`.
- Handlers are plain `async fn` with **no `self`**. A controller is a stateless namespace;
  everything a handler needs arrives through extractors (see below). This is enforced at
  compile time — a `self` receiver is a clear error.

Mount it when you build the app:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    App::builder().controllers().run().await
}
```

`controllers()` mounts **every** `#[controller]` linked into the binary — there is no
list to keep in sync. To mount one explicitly instead, use
`App::builder().controller::<HomeController>()`.

## HTTP verbs

Each method attribute maps to one verb: `#[get]`, `#[post]`, `#[put]`, `#[patch]`,
`#[delete]`, `#[head]`, `#[options]`. Several methods may share a path:

```rust
#[controller("/api/todos")]
impl TodoController {
    #[get("/")]              async fn index(/* ... */) { /* ... */ }
    #[post("/")]             async fn store(/* ... */) { /* ... */ }
    #[get("/{id}")]          async fn show(/* ... */)  { /* ... */ }
    #[patch("/{id}")]        async fn update(/* ... */) { /* ... */ }
    #[delete("/{id}")]       async fn destroy(/* ... */) { /* ... */ }
}
```

## Path parameters

Dynamic segments use `{name}` (axum syntax) and are pulled out with the `Path` extractor:

```rust
#[get("/{id}")]
async fn show(db: Db, Path(id): Path<i64>) -> Result<Json<Todo>> {
    Ok(Json(Todo::find_or_404(&db, id).await?))
}
```

`Path<(i64, String)>` extracts several segments in order.

## Query strings

Bind the query string to a `Deserialize` struct with `Query`:

```rust
#[derive(Deserialize)]
struct Paging {
    page: Option<u64>,
    per_page: Option<u64>,
}

#[get("/")]
async fn index(db: Db, Query(paging): Query<Paging>) -> Result<Json<Page<Todo>>> {
    let page = Todo::query()
        .order_by_desc(Todo::ID)
        .fetch_page(&db, paging.page.unwrap_or(1), paging.per_page.unwrap_or(20))
        .await?;
    Ok(Json(page))
}
```

## Request bodies

`Json<T>` deserialises a JSON body; `Form<T>` deserialises a URL-encoded form body. For
input you want to validate, reach for `Validated<Json<T>>` / `Validated<Form<T>>` instead
— see [Requests & Validation](validation.md).

```rust
#[post("/")]
async fn store(db: Db, Json(input): Json<CreateTodo>) -> Result<(StatusCode, Json<Todo>)> {
    let todo = Todo::new(input.title.trim()).insert(&db).await?;
    Ok((StatusCode::CREATED, Json(todo)))
}
```

## Common extractors

Anything that implements axum's `FromRequestParts`/`FromRequest` can be a handler
argument. The ones you will use most:

| Extractor | Gives you |
|---|---|
| `Path<T>` | Path parameters |
| `Query<T>` | The query string, deserialised |
| `Json<T>` / `Form<T>` | The request body |
| `Db` | A database connection (`orm` feature) |
| `Ctx` | The shared `AppContext` (config, services) |
| `Inject<T>` | One registered service by type |
| `Auth<U>` / `OptionalAuth<U>` | The authenticated user (`auth` feature) |
| `Flash`, `Csrf`, `Session` | Session helpers (`sessions` feature) |

Order does not matter, except that a body extractor (`Json`/`Form`/`Validated`) consumes
the request and must come **last** in the argument list.

## Responses

A handler returns anything that implements `IntoResponse`:

- `&'static str` / `String` → `text/plain`
- `Json<T>` → `application/json`
- `Html<String>` or `View<T>` → `text/html` (see [Views](views.md))
- `Redirect::to("/path")` → a 303 redirect
- `StatusCode`, or a tuple like `(StatusCode::CREATED, Json(todo))`, to set the status
- `Result<T>` where the error is the framework [`Error`](responses.md) — `?` just works

```rust
#[delete("/{id}")]
async fn destroy(db: Db, Path(id): Path<i64>) -> Result<StatusCode> {
    Todo::find_or_404(&db, id).await?.delete(&db).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

## Named routes

Give a route a `name` and it appears in the route table and (once the view layer's URL
helper lands) in generated links:

```rust
#[get("/{id}", name = "todos.show")]
async fn show(/* ... */) { /* ... */ }
```

List every controller route, its name, handler and middleware:

```bash
cargo run -- routes
```

```text
METHOD  PATH             NAME          HANDLER                 MIDDLEWARE
GET     /api/todos       todos.index   TodoController::index   Throttle::per_minute(600)
POST    /api/todos       todos.store   TodoController::store   Throttle::per_minute(600)
GET     /api/todos/{id}  todos.show    TodoController::show     Throttle::per_minute(600)
```

## SSR and API from one controller

Because a handler just returns an `IntoResponse`, the same controller can serve HTML
pages and a JSON API side by side — pick the return type per route:

```rust
#[controller("/posts")]
impl PostController {
    #[get("/")]            async fn index() -> Result<View<PostsIndex>> { /* HTML */ }
    #[get("/api/posts")]   async fn api_index() -> Result<Json<Vec<Post>>> { /* JSON */ }
}
```

Errors are rendered to match the client automatically (JSON for API callers, an HTML page
for browsers) — see [Responses & Errors](responses.md).

## Attaching middleware

`#[middleware(expr)]` on a method wraps that one handler; on the `impl` it wraps the whole
controller. See [Middleware & Filters](middleware.md).

```rust
#[controller("/admin")]
#[middleware(Throttle::per_minute(60))]   // whole controller
impl AdminController {
    #[delete("/users/{id}")]
    #[authorize("user.delete")]            // one route
    async fn destroy(/* ... */) { /* ... */ }
}
```

> **Under the hood:** `#[controller]` generates ordinary axum router code — run
> `cargo expand` to see it. There is no runtime reflection; the macro only moves the
> route declaration next to the handler.
