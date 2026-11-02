# Migrations (Code-First)

Rustali's headline database feature: change an entity, run `migrate:diff`, and get a
reviewed migration. You still own and read every migration; the diff just writes the first
draft from what changed.

## How it works

Three things take part:

1. **Your entities** — `#[derive(Entity)]` registers each one's schema at compile time.
2. **A snapshot** — `db/schema.snapshot.json` records the schema as it was last applied.
   It is committed to version control.
3. **`migrate:diff`** — compares the entities against the snapshot, writes a migration for
   the difference, and updates the snapshot.

## Enabling migrations

Collect your migrations in a `Migrator` and register it on the builder with
`.migrations::<Migrator>()`. This adds the `migrate*` commands to your app.

```rust
// src/migrations.rs
use rustali::migrate::prelude::*;

mod m20261101_000001_create_posts;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20261101_000001_create_posts::Migration)]
    }
}
```

```rust
// src/main.rs
App::builder()
    .provider(DatabaseProvider)
    .migrations::<migrations::Migrator>()
    .controllers()
    .run()
    .await
```

## The commands

Run them through your app binary:

| Command | Does |
|---|---|
| `cargo run -- migrate` | Apply pending migrations. |
| `cargo run -- migrate:status` | Show which migrations are applied. |
| `cargo run -- migrate:rollback` | Roll back the last batch (`--step N` for more). |
| `cargo run -- migrate:fresh` | Drop all tables and re-apply everything. |
| `cargo run -- migrate:refresh` | Roll back all, then re-apply. |
| `cargo run -- migrate:reset` | Roll back everything. |
| `cargo run -- migrate:diff "name"` | Generate a migration from entity changes. |

`fresh`, `refresh` and `reset` are destructive; they refuse to run under
`APP_ENV=production` unless you set `RUSTALI_FORCE=1`.

## The code-first loop

Change a field on an entity:

```rust
#[derive(Entity, Clone)]
#[table("posts")]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub published: bool,   // <-- new column
    // ...
}
```

Diff it against the snapshot:

```bash
cargo run -- migrate:diff "add published to posts"
```

```text
Detected 1 change(s):
   -  alter_posts
Wrote db/migrations/m20261101_120000_add_published_to_posts.rs
Updated db/schema.snapshot.json
```

The generated file is ordinary, readable `sea-orm-migration` code — a `Table::alter()` with
`add_column`, a matching `down`, and the `DeriveIden` enum. Review and edit it freely;
nothing executes a generated string.

```rust
// db/migrations/m20261101_120000_add_published_to_posts.rs (generated)
async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
        .alter_table(Table::alter().table(Posts::Table)
            .add_column(ColumnDef::new(Posts::Published).boolean().not_null())
            .to_owned())
        .await?;
    Ok(())
}
```

From an empty snapshot the first diff emits full `create_table` migrations for every
entity — so bootstrapping a new project is: define your entities, run one `migrate:diff
"initial schema"`, review, and `migrate`.

Every generated migration has a matching `down`, so the change is reversible. For the column
above, the `down` drops it again:

```rust
// the generated down for "add published to posts"
async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
        .alter_table(Table::alter().table(Posts::Table)
            .drop_column(Posts::Published)
            .to_owned())
        .await?;
    Ok(())
}
```

`migrate:rollback` runs these, so you can undo the last batch during development. Review the
generated `down` as carefully as the `up` — it is what a rollback will execute against real
data.

## A typical loop

The day-to-day rhythm once migrations are wired in:

1. Edit an entity (add a field, change a type).
2. `cargo run -- migrate:diff "describe the change"` — writes a migration and updates the
   snapshot.
3. **Read the generated file.** Adjust it if the diff's guess is not quite what you meant
   (a default value, an index, a data backfill).
4. Move it into `src/migrations/` and register it (see the gap below).
5. `cargo run -- migrate` to apply it locally; commit the migration and the updated
   snapshot together.

In production you ship the committed migrations and run `cargo run -- migrate` (or your
binary's `migrate`) on deploy. You do **not** run `migrate:diff` in production — diffing is a
development-time authoring step; applying is the only thing that touches a production
database.

## Destructive changes

Dropping a column or table, changing a type, or tightening `NULL`→`NOT NULL` can lose
data, so `migrate:diff` **refuses** them by default:

```text
1 destructive change(s) detected (drop / type change / NOT NULL). Re-run with
--allow-destructive once you have reviewed them.
```

Re-run with `--allow-destructive` once you have looked at what it will do.

## Wiring a generated migration in

::: warning Known gap
`migrate:diff` writes to `db/migrations/`, but your app compiles migrations from
`src/migrations/`. For now, move the generated file into `src/migrations/`, add its
`mod m...;` line, and list `Box::new(m...::Migration)` in your `Migrator`. Automatic
placement and wiring is on the roadmap.
:::

## Writing a migration by hand

You never have to use the diff. `rustali make:migration create_posts` scaffolds an empty
migration you fill in with the SeaORM schema builder (`Table::create()`, `ColumnDef`,
indexes, foreign keys). Everything in [`sea-orm-migration`] is available through
`rustali::migrate::prelude::*`.

[`sea-orm-migration`]: https://docs.rs/sea-orm-migration

## What the diff understands today

Tables and columns (add/drop), column type changes, nullability, uniqueness, and string
length. Unknown Rust types become a `TEXT` column with a `FIXME` comment rather than being
silently dropped, so a diff is never lossy by surprise. Relations on the struct (foreign keys
beyond a plain `*_id` column) are a future refinement.

### Renames need a human

The diff has no way to tell a rename from a delete-and-create: renaming `title` to `heading`
looks to it like *drop `title`, add `heading`* — which would throw the old data away. So when
you rename a column, do not trust the generated drop/add. Either edit the generated migration
to a real `rename_column`, or write that migration by hand. This is exactly why the diff is a
*draft you review*, not an automatic apply: the one change it cannot infer is the one that
would lose data silently.

### Soft delete is just a column

The `deleted_at` column that scaffolded models carry is, to the diff, an ordinary nullable
timestamp — it is migrated like any other column. The soft-delete *behaviour* (stamping it on
`delete`, hiding trashed rows) is a runtime setting, `SOFT_DELETE` in `.env`,
not something the migration or schema knows about. See
[Base Entity & Soft Delete](base-entity).
