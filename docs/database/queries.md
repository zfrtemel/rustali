# Queries & Pagination

`Todo::query()` returns a typed `Query<Todo>` builder. It forwards SeaORM's familiar
filter, order and select methods, but its terminal methods return **your struct**, not a
SeaORM model — so you stay in plain-Rust-value land.

## The `Db` extractor

Handlers get a connection by asking for `Db`. It implements SeaORM's `ConnectionTrait`, so
`&db` goes straight into any query method:

```rust
#[get("/")]
async fn index(db: Db) -> Result<Json<Vec<Todo>>> {
    Ok(Json(Todo::all(&db).await?))
}
```

Outside a handler (in a job, a command) use `ctx.db()?`, which returns the shared
`Database`; pass `&*ctx.db()?`.

## Building a query

Filter and order with the typed column constants the derive generated:

```rust
let done = Todo::query()
    .filter(Todo::DONE.eq(true))
    .order_by_desc(Todo::CREATED_AT)
    .limit(10)
    .fetch_all(&db)
    .await?;
```

`filter`, `order_by_asc` / `order_by_desc`, `limit`, `offset` and the other builder
methods come from SeaORM (`ColumnTrait`, `QueryOrder`, `QuerySelect`); the column
constants (`Todo::DONE`, `Todo::CREATED_AT`) are generated for each field. Combine
conditions with `.filter(...)` repeatedly or SeaORM's `Condition`.

## Fetching

| Terminal method | Returns |
|---|---|
| `fetch_all(&db)` | `Vec<Todo>` |
| `fetch_one(&db)` | `Option<Todo>` (first match) |
| `fetch_one_or_404(&db)` | `Todo` or `Err(Error::NotFound)` |
| `count(&db)` | `u64` |
| `exists(&db)` | `bool` |
| `fetch_page(&db, page, per_page)` | `Page<Todo>` |

```rust
let user = User::query()
    .filter(User::EMAIL.eq(&email))
    .fetch_one(&db)
    .await?;                       // Option<User>

let already = User::query()
    .filter(User::EMAIL.eq(&email))
    .exists(&db)
    .await?;                       // bool
```

## Pagination

`fetch_page` returns a `Page<T>` carrying the rows and the numbers a UI or API client
needs. It is `Serialize`, so an API can return it directly.

```rust
#[derive(Deserialize)]
struct Paging { page: Option<u64>, per_page: Option<u64> }

#[get("/")]
async fn index(db: Db, Query(p): Query<Paging>) -> Result<Json<Page<Todo>>> {
    let page = Todo::query()
        .order_by_desc(Todo::ID)
        .fetch_page(&db, p.page.unwrap_or(1), p.per_page.unwrap_or(20).min(100))
        .await?;
    Ok(Json(page))
}
```

`Page<T>` fields:

| Field | Meaning |
|---|---|
| `items` | the rows on this page |
| `page` | 1-based current page |
| `per_page` | requested page size |
| `total` | total rows across all pages |
| `last_page` | number of the last page (≥ 1) |

It also has `has_next()` / `has_prev()` helpers and `map(f)` to transform items while
keeping the paging numbers (handy to turn entities into a view model).

```json
{ "items": [ /* ... */ ], "page": 1, "per_page": 20, "total": 57, "last_page": 3 }
```

## Transactions

Use SeaORM's transaction API on the connection:

```rust
use rustali::orm::sea_orm::TransactionTrait;

db.transaction::<_, (), Error>(|tx| Box::pin(async move {
    some.update(tx).await?;
    other.insert(tx).await?;
    Ok(())
})).await?;
```

Every entity method and `Query` terminal accepts the transaction handle (`tx`) wherever it
takes `&db`.

## Escaping to SeaORM

When the builder does not cover something, `Query::into_select()` returns the underlying
SeaORM `Select<E::Sea>` to drive directly, and `rustali::orm::sea_orm` re-exports the whole
SeaORM API. Raw SeaORM calls that return a `DbErr` can be mapped with `.or_db()?`
(`DbResultExt`), which turns a missing row into `Error::NotFound` and anything else into
`Error::Database`.

Next: [Migrations (Code-First)](migrations).
