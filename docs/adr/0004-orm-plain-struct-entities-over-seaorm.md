# ADR-0004: Plain-struct entities over SeaORM, typed `Query<E>` instead of a back-link trait

- **Status:** Accepted
- **Date:** 2026-11-01

## Context

The goal is plain-struct entities: one struct per table, no visible
`Model`/`ActiveModel`/`Relation` triplet. SeaORM 2.0 is the engine (mature,
async, multi-backend, has a migration runner), but its native shape is exactly that
triplet. We also need a path from "struct" to "schema" for code-first migrations
(Faz 6).

## Decision

1. **The user's struct is the entity.** `#[derive(Entity)]` generates a hidden
   `<snake>_entity` module containing the SeaORM `Model`, `ActiveModel`, `Entity`,
   `Column`, `PrimaryKey` and `Relation`, plus `From` conversions in both directions,
   an implementation of `rustali_orm::Entity` (table name, column metadata, active
   model builders) and inherent CRUD helpers (`find`, `find_or_404`, `all`, `count`,
   `insert`, `update`, `delete`, typed column constants).
2. **Conventions over attributes.** A field named `id` is the key when no `#[key]`
   is given; integer keys auto-increment; `Option<T>` is nullable; `created_at` /
   `updated_at` of a `DateTime` type are maintained by an `ActiveModelBehavior`
   hook. Attributes (`#[table]`, `#[key(auto = false)]`, `#[column(name, unique,
   max_len)]`, `#[skip]`) exist for the exceptions.
3. **Queries are `Query<E>`**, a newtype around `Select<E::Sea>` that implements
   SeaORM's `QueryFilter`, `QueryOrder` and `QuerySelect` by delegation and offers
   `fetch_all`, `fetch_one`, `fetch_one_or_404`, `count`, `exists`, `fetch_page`
   returning plain structs and a serialisable `Page<E>`.
4. **SeaORM stays reachable.** `rustali_orm::sea_orm` is re-exported;
   `Query::into_select()` and `DbResultExt::or_db()` are the escape hatches.
5. **Layering.** `rustali-core` never depends on a database crate. The ORM registers
   `Database` through the `Provider` mechanism; `Error::Database` carries driver errors
   as a boxed source; `DbErr::RecordNotFound` maps to `Error::NotFound`.
6. **Generated module visibility equals the struct's visibility.**

## What we tried and rejected

- *A `Row` back-link trait on the generated SeaORM entity* (`impl Row for Entity {
  type Entity = UserStruct; }`) so that `Select<Sea>` could infer the struct type.
  Rejected: SeaORM's generated `Entity` is nominally `pub`, so a private user struct
  in that associated type is a hard error (E0446), regardless of module privacy.
  `Query<E>` carries the type explicitly and has no such problem.
- *Glob-importing SeaORM's prelude into the generated module alongside
  `use super::*`.* The prelude exports `DateTime`, `Uuid`, `Json`…, which collide
  with the user's imports. The derive now adds explicit `use super::<Ident>;` for
  every single-segment type name used by the fields (explicit imports beat globs),
  while still globbing the prelude because `DeriveEntityModel` needs it in scope.
- *`Db` as a thin wrapper without `ConnectionTrait`.* Users would write `&*db`
  everywhere. `Db` and `Database` now delegate the five required `ConnectionTrait`
  methods so `&db` works in every SeaORM API.

## Consequences

- Handlers read fluently: `Todo::query().filter(..).order_by_desc(..).fetch_page(&db, 1, 20)`.
- Column metadata (`Entity::COLUMNS`) is already available for the Faz 6 diff engine.
- Relations (`has_many`, `belongs_to`) are not yet expressed on the struct; they come
  with Faz 3/6 once the query layer settles.
- A macro bug surfaces as an error on the user's `#[derive(Entity)]` line; `trybuild`
  tests pin the messages for the known misuse cases.
