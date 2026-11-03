# Model Methods

`#[derive(Entity)]` gives every model a set of methods for the common database operations —
reading rows, creating, updating and deleting. This page walks through each one on its own:
what it takes, what it returns, and when to reach for it.

Every method takes a database connection as its first (or, for the instance methods, implicit
`self` plus a) argument. In a handler that connection is the [`Db`](queries#the-db-extractor)
extractor; elsewhere it is `&*ctx.db()?`. All of them are `async` and return
[`Result`](../basics/responses#errors-are-values), so you `.await?` them.

The examples below use this model:

```rust
use rustali::{orm::chrono::{DateTime, Utc}, prelude::*};

#[entity]
#[table("todos")]
pub struct Todo {
    pub id: i64,
    #[column(max_len = 200)]
    pub title: String,
    pub done: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

## Retrieving models

### `find`

```rust
Todo::find(&db, id) -> Result<Option<Todo>>
```

Fetch one row by primary key. Returns `None` when no row has that key — not an error, so you
decide what a miss means:

```rust
match Todo::find(&db, 1).await? {
    Some(todo) => println!("got {}", todo.title),
    None => println!("no such todo"),
}
```

### `find_or_404`

```rust
Todo::find_or_404(&db, id) -> Result<Todo>
```

Like `find`, but a miss is `Err(Error::NotFound)` instead of `None`. In a handler, `?` turns
that straight into a 404 response — which is exactly what you want for a "show this record"
route:

```rust
#[get("/{id}")]
async fn show(db: Db, Path(id): Path<i64>) -> Result<Json<Todo>> {
    Ok(Json(Todo::find_or_404(&db, id).await?))   // 404 if missing
}
```

### `all`

```rust
Todo::all(&db) -> Result<Vec<Todo>>
```

Every row, ordered by primary key. Good for small tables; for anything that can grow, filter
or paginate with [`query`](#query) instead of loading the whole table into memory.

```rust
let todos = Todo::all(&db).await?;
println!("{} todos", todos.len());
```

### `count`

```rust
Todo::count(&db) -> Result<u64>
```

The number of rows in the table, counted by the database — it does not load any rows.

```rust
if Todo::count(&db).await? == 0 {
    println!("nothing to do");
}
```

### `query`

```rust
Todo::query() -> Query<Todo>
```

Start a typed [query builder](queries) when you need filtering, ordering, limits or
pagination — anything beyond "by id" or "all". It is synchronous (it builds the query); the
terminal method that runs it is the `async` one:

```rust
let open = Todo::query()
    .filter(Todo::DONE.eq(false))
    .order_by_desc(Todo::CREATED_AT)
    .limit(20)
    .fetch_all(&db)
    .await?;
```

See [Queries & Pagination](queries) for the full builder.

## Creating models

### `insert`

```rust
todo.insert(&db) -> Result<Todo>
```

Insert the value as a new row and return the **stored** row. The returned value has the real
generated key and timestamps filled in, so use what `insert` hands back, not the value you
passed:

```rust
let todo = Todo {
    id: 0,                              // ignored — the DB assigns it
    title: "write docs".into(),
    done: false,
    created_at: DateTime::<Utc>::MIN_UTC,  // ignored — set on insert
    updated_at: DateTime::<Utc>::MIN_UTC,
};
let todo = todo.insert(&db).await?;     // todo.id and timestamps are now real
```

An auto-incrementing key and the `created_at` / `updated_at` fields are managed for you (see
[Entities → Timestamps](entities#timestamps)), which is why their placeholder values above do
not matter.

## Updating models

### `update`

```rust
todo.update(&db) -> Result<Todo>
```

Write every column of the value back to its row (matched by primary key) and return the
stored row. `updated_at` is refreshed automatically. The usual shape is load, change fields,
save:

```rust
let mut todo = Todo::find_or_404(&db, id).await?;
todo.done = true;
let todo = todo.update(&db).await?;     // updated_at bumped
```

`update` writes **all** columns, not just the ones you changed — it is a whole-row update, so
the value you save should be a complete, current row (which it is, if you loaded it first).

## Deleting models

### `delete`

```rust
todo.delete(&db) -> Result<()>
```

Delete the row (matched by primary key). Consumes the value, since it no longer refers to
anything:

```rust
let todo = Todo::find_or_404(&db, id).await?;
todo.delete(&db).await?;
```

To delete without loading first, run a delete through the [query builder](queries) or drop to
SeaORM — `delete` here is the "I already have the row" path.

## Relations

Related rows are loaded by their own generated methods — `todo.comments(&db)` and the eager
`Todo::with_comments(todos, &db)` — one per relation you declare. Those are covered in full
under [Relationships](relations/).

## Typed column handles

Alongside the methods, the derive generates a constant per column — `Todo::ID`,
`Todo::TITLE`, `Todo::DONE`, … — for use in queries (`Todo::DONE.eq(false)`). They are how a
query refers to a column by a checked name rather than a string. See
[Queries](queries#building-a-query).

## Dropping to SeaORM

These methods cover the everyday cases. For anything they do not — bulk updates, upserts,
custom joins — [`Query::into_select()`](queries#escaping-to-seaorm) hands you the raw SeaORM
`Select`, and `rustali::orm::sea_orm` re-exports the whole SeaORM API. There is no second
query language to learn.
