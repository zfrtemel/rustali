# Base Entity & Soft Delete

Most tables share the same handful of columns: a primary key, creation and update
timestamps, and — if you keep deleted rows around — a `deleted_at` marker. Rustali gives
these a shared shape through two traits, `BaseEntity` and `SoftDelete`, and the `make:model`
scaffold writes them for you, so every model starts from the same base — the idea you would
reach for a base class for in other languages, expressed as traits because Rust has no inheritance.

## The scaffolded model

`rustali make:model Post` generates a model that already carries the common fields and
implements both traits:

```rust
use rustali::{orm::chrono::{DateTime, Utc}, prelude::*};
use serde::Serialize;

#[derive(Entity, Debug, Clone, Serialize)]
#[table("posts")]
pub struct Post {
    pub id: i64,
    // Add your own fields here.
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl BaseEntity for Post {
    type Id = i64;
    fn id(&self) -> Self::Id { self.id }
    fn created_at(&self) -> DateTime<Utc> { self.created_at }
    fn updated_at(&self) -> DateTime<Utc> { self.updated_at }
}

impl SoftDelete for Post {
    fn deleted_at(&self) -> Option<DateTime<Utc>> { self.deleted_at }
}
```

You add your own columns between the key and the timestamps. The timestamps are managed by
`#[derive(Entity)]`'s conventions (`created_at` on insert, `updated_at` on every save — see
[Entities → Timestamps](entities#timestamps)), so you never set them by hand.

## `BaseEntity`

`BaseEntity` is a small trait of accessors — `id()`, `created_at()`, `updated_at()` — shared
by every model with those fields. Rust has no base class, so "a base model" is a trait a
struct implements rather than a parent it extends; the fields stay on the struct, the trait
gives shared, type-checked access to them.

Its value is **generic code**: a function can take "any base model" without naming the
concrete type.

```rust
fn audit_line<T: BaseEntity<Id = i64>>(row: &T) -> String {
    format!("#{} created {}", row.id(), row.created_at())
}

audit_line(&post);     // works for any model implementing BaseEntity
```

Implement it by hand (as above) or let the scaffold write it. You only need it on models
where shared handling pays off — it is not required for an entity to work.

## `SoftDelete`

`SoftDelete` extends `BaseEntity` for models that carry a nullable `deleted_at`. It adds one
accessor and a convenience method:

| Method | Returns |
|---|---|
| `deleted_at()` | `Option<DateTime<Utc>>` — when the row was trashed, or `None` while live |
| `is_trashed()` | `true` when `deleted_at` is set |

```rust
if post.is_trashed() {
    // the row is soft-deleted
}
```

The trait is just the accessor side. The **behaviour** — turning a `delete` into a stamp and
hiding trashed rows from queries — is driven by config, described next.

## Soft delete behaviour

Soft delete is a project-wide policy, off by default. Turn it on in `.env`:

```bash
# .env
SOFT_DELETE=true
```

With it on, for any model that has a `deleted_at` column:

- **`delete` stamps instead of removing.** `post.delete(&db)` sets `deleted_at = now()` and
  leaves the row in the table, rather than issuing a `DELETE`.
- **Queries hide trashed rows.** `Post::query()`, and therefore `find`, `all` and `count`,
  start filtered to `deleted_at IS NULL`, so trashed rows do not show up in ordinary reads.

```rust
// with soft_delete = true
let post = Post::find_or_404(&db, id).await?;
post.delete(&db).await?;                       // deleted_at stamped, row kept

assert!(Post::find(&db, id).await?.is_none());  // hidden from normal queries
```

With `soft_delete = false` (the default), `delete` removes the row and queries are
unfiltered — the `deleted_at` column is then just data you can use yourself.

The switch is read once at startup from config, so it is a single consistent policy for the
whole app, not a per-call decision. A model without a `deleted_at` column is unaffected either
way — its `delete` is always a real delete.

## Seeing trashed rows

When soft delete is on and you need the trashed rows too — a "recently deleted" view, an
admin screen — drop to SeaORM for that one query, since the entity's `query()` filters them
out:

```rust
use rustali::orm::sea_orm::EntityTrait;

// every row, trashed included
let all = post_entity::Entity::find().all(&db).await?;
```

This is deliberately explicit: the default path is safe (trashed rows stay hidden), and
seeing them is a conscious step.

Next: [Migrations (Code-First)](migrations).
