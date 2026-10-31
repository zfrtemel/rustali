# ADR-0007: Code-first migrations by diffing entities against a snapshot

- **Status:** Accepted
- **Date:** 2026-11-01

## Context

The headline differentiator (VISION §4.5, ADR-0004 groundwork): change an entity,
run `rustali migrate:diff`, get a reviewed migration — EF Core's code-first flow, which
no Rust ORM offers out of the box. SeaORM gives a migration *runner* but no diff; the
diff is ours to build.

## Decision

1. **A normalised, backend-neutral schema model** lives in `rustali_orm::schema`:
   `Schema` → `Table` → `Column` with an abstract `ColumnType`. It is pure and
   I/O-free, and serialises to JSON. `ColumnType::from_rust` maps the Rust type string
   that `#[derive(Entity)]` already records (`ColumnMeta.rust_type`) to a semantic type,
   so the diff compares meaning, not spelling.
2. **The desired schema is assembled from entities at compile time.** `#[derive(Entity)]`
   submits an `EntityRegistration` to an `inventory` collection (like controllers,
   ADR-0005); `desired_schema()` builds a `Schema` from every entity linked in. No list
   to maintain.
3. **The current schema is a snapshot file**, `db/schema.snapshot.json`, committed to
   version control. Absent file = empty schema (first diff creates everything).
4. **`Schema::diff`** produces ordered `SchemaChange`s (creates, then alters, then
   drops; within a table: adds, modifies, drops). Each change reports `is_destructive()`
   — drops, type changes, and NULL→NOT NULL. Widening to nullable is safe.
5. **`migrate:diff "name"`** (in `rustali-migrate`): load snapshot, build desired,
   diff, print a summary (destructive marked `[!]`), **refuse destructive changes
   without `--allow-destructive`**, render a migration file to `db/migrations/`, and
   rewrite the snapshot. It reminds the user to register the module in their `Migrator`.
6. **Generated migrations are ordinary, readable `sea-orm-migration` code** —
   `Table::create()/alter()`, `ColumnDef`, a `DeriveIden` enum, and an inverse `down`.
   Nothing executes a generated string; the file is yours to edit. (`down` for a
   dropped column leaves a `TODO`, since the original definition is no longer known.)

## Consequences

- Proven on the `todo-api` entity: empty→create, add nullable column→single
  `ADD COLUMN` (not a recreate), remove column→refused then generated with the flag.
- **Known gap:** `migrate:diff` writes to `db/migrations/`, but examples compile
  migrations from `src/migrations/`. Until the CLI generators (Faz 8) place and wire
  files for you, the generated file is a starting point you move into `src/` and list
  in `Migrator`. Documented, not yet automated.
- **Type mapping is best-effort.** Unknown Rust types become `ColumnType::Unknown`,
  which the generator renders as `.text()` with a `FIXME`, so a diff never silently
  drops a column. Enums, custom types and relations (`belongs_to` columns are plain
  FKs today) will be refined as real apps exercise them.
- Rename detection is not attempted: a renamed column reads as drop+add. A future
  `--rename old:new` hint can suppress that (VISION noted this).
- The snapshot is the source of truth for "what the DB looks like"; editing a
  generated migration by hand without updating the entity will drift from it. This is
  the same contract EF Core has.
