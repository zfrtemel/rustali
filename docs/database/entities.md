# Entities

An entity is a **plain struct** you annotate with `#[entity]` (which expands to
`#[derive(Entity, Debug, Clone, Serialize)]`). The struct is what
you pass around, serialise and render; the derive generates the [SeaORM] plumbing
(`Model`, `ActiveModel`, columns, primary key) into a hidden module you never touch — one
readable type, no visible ORM boilerplate.

[SeaORM]: https://www.sea-ql.org/SeaORM/

## Defining one

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

That is the whole model. `#[derive(Entity)]` also wires it into the schema registry so
[`migrate:diff`](migrations) can see it.

## Conventions

The derive leans on conventions so the common case needs no attributes:

- A field named **`id`** is the primary key. An integer key **auto-increments**.
- **`Option<T>`** fields are nullable; everything else is `NOT NULL`.
- Fields named **`created_at`** / **`updated_at`** of a `DateTime` type are set
  automatically — `created_at` on insert, `updated_at` on every save.
- The table name defaults to the snake_case struct name + `s`; override with `#[table]`.

## Attributes

### Container

| Attribute | Effect |
|---|---|
| `#[table("name")]` | Table name. |
| `#[entity(crate = "rustali_orm")]` | Path to the ORM crate, for generated code (rarely needed). |
| `#[has_many(...)]` | A one-to-many relation. See [Relationships](relations/). |
| `#[belongs_to(...)]` | The child side of a one-to-many relation. |
| `#[has_one(...)]` | A one-to-one relation. |
| `#[many_to_many(...)]` | A many-to-many relation through a junction. |

### Field

| Attribute | Effect |
|---|---|
| `#[key]` | Mark the primary key (when it is not named `id`). |
| `#[key(auto = false)]` | Non-auto-incrementing key. |
| `#[column(name = "col")]` | Map to a differently named column. |
| `#[column(unique)]` | Add a `UNIQUE` constraint. |
| `#[column(max_len = N)]` | `VARCHAR(N)` for a string. |
| `#[column(index)]` | Record that the column should be indexed (for schema tooling). |
| `#[skip]` | Not stored; must implement `Default` (filled on load). |

```rust
#[entity]
#[table("users")]
pub struct User {
    pub id: i64,
    #[column(unique)]
    pub email: String,
    pub password_hash: String,
    #[skip]
    pub transient_token: Option<String>,
}
```

## Generated methods

`#[derive(Entity)]` gives the struct a CRUD surface and typed column constants. All take a
connection — the [`Db`](queries#the-db-extractor) extractor, or `&*ctx.db()?`.

| Method | Does |
|---|---|
| `Todo::query()` | Start a [`Query`](queries) builder. |
| `Todo::find(&db, id)` | `Option<Todo>` by primary key. |
| `Todo::find_or_404(&db, id)` | `Todo` or `Err(Error::NotFound)`. |
| `Todo::all(&db)` | Every row, by primary key. |
| `Todo::count(&db)` | Row count. |
| `todo.insert(&db)` | Insert; returns the stored row (with generated id + timestamps). |
| `todo.update(&db)` | Update every column; returns the stored row. |
| `todo.delete(&db)` | Delete this row. |
| `Todo::ID`, `Todo::TITLE`, ... | Typed column handles for queries. |

Each of these is covered method by method, with usage and return types, in
[Model Methods](model-methods).

```rust
// create
let todo = Todo::new("write docs").insert(&db).await?;

// read
let one = Todo::find_or_404(&db, todo.id).await?;

// update
let mut t = one;
t.done = true;
let t = t.update(&db).await?;

// delete
t.delete(&db).await?;
```

(`Todo::new(...)` above is an ordinary constructor you write — the derive does not
generate it. The examples define a small `new` that sets defaults and leaves `id`/
timestamps to `insert`.)

## Using an entity in a handler

```rust
#[get("/{id}")]
async fn show(db: Db, Path(id): Path<i64>) -> Result<Json<Todo>> {
    Ok(Json(Todo::find_or_404(&db, id).await?))
}
```

Because the struct derives `Serialize`, returning `Json<Todo>` just works.

## Relationships

Links to other tables — a post's comments, a comment's post, a post's tags — are declared
with container attributes (`#[has_many]`, `#[belongs_to]`, `#[has_one]`,
`#[many_to_many]`) and loaded with generated, explicit methods. See
[Relationships](relations/) for the full guide.

## What the derive generates

Nothing here is reflection or runtime magic — `#[derive(Entity)]` expands to ordinary code
you can read with `cargo expand`. For a `Todo` struct it emits, into a hidden module named
`todo_entity`:

- The **SeaORM entity**: `Model`, `ActiveModel`, `Column`, `PrimaryKey`, `Relation` — the
  plumbing SeaORM needs, kept out of sight.
- **`From` conversions** both ways: `From<todo_entity::Model> for Todo` and
  `From<Todo> for todo_entity::Model`. Query results come back as your struct because the
  terminal methods call `.into()` for you.
- An **`impl Entity for Todo`** carrying the table name, column metadata (`COLUMNS`), the
  key accessor and the active-model builders — this is what the schema registry and
  [migrations](migrations) read.
- The **inherent methods** in the table above, the typed column constants, and any
  [relation](relations/) accessors.

Because the mapping is plain `From`, you can always convert between your struct and the raw
SeaORM model when you need to drop down a level.

## Timestamps

When a struct has a `created_at` and/or `updated_at` field of a `DateTime` type, the derive
generates a save hook (SeaORM's `ActiveModelBehavior::before_save`):

- `created_at` is set once, on **insert**, if it has not been set.
- `updated_at` is set on **every** save (insert and update).

So you can leave both at any placeholder value when building a row — `insert` returns the
stored row with the real timestamps filled in. On insert, both fields are left for the hook
rather than sent from your struct; on update, `updated_at` is always refreshed. Omit the
fields entirely and you simply have no timestamps; the hook is only generated when they
exist.

## Shared base fields & soft delete

`rustali make:model` gives every new model the conventional `id` / `created_at` /
`updated_at` / `deleted_at` fields plus the `BaseEntity` and `SoftDelete` trait impls — a
shared base model. With soft delete enabled in config, `delete` then stamps
`deleted_at` instead of removing the row and queries hide trashed rows. See
[Base Entity & Soft Delete](base-entity).

## Dropping to SeaORM

The generated column constants implement SeaORM's traits, and
[`Query::into_select()`](queries) hands you the raw `Select` — so anything the sugar does
not cover, you can still do with SeaORM directly through `rustali::orm::sea_orm`. There is
no second query language to learn.

Next: [Queries & Pagination](queries).
