# Migrations (Code-First)

Rustali's headline database feature: change an entity, run `migrate:diff`, and get a
reviewed migration — EF Core's code-first flow. You still own and read every migration;
the diff just writes the first draft from what changed.

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
entity.

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
length. Renames read as drop + add for now. Unknown Rust types become a `TEXT` column with
a `FIXME` comment rather than being silently dropped, so a diff is never lossy by surprise.
Relations on the struct (foreign keys beyond a plain `*_id` column) are a future
refinement.
