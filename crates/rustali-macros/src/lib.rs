//! Procedural macros for the Rustali web framework.
//!
//! This crate is an implementation detail re-exported by `rustali`; it must only
//! depend on the proc-macro toolkit. Generated code refers to the framework through a
//! configurable path (default `::rustali`) so it works whether the user depends on
//! the meta crate or on the individual crates.

mod controller;
mod entity;

use proc_macro::TokenStream;

/// Turn a plain struct into a Rustali entity backed by a generated SeaORM entity.
///
/// ```ignore
/// use rustali::prelude::*;
///
/// #[derive(Entity, Debug, Clone, Serialize)]
/// #[table("todos")]
/// pub struct Todo {
///     #[key]
///     pub id: i64,
///     #[column(max_len = 200)]
///     pub title: String,
///     pub done: bool,
///     pub created_at: DateTime<Utc>,
///     pub updated_at: DateTime<Utc>,
/// }
/// ```
///
/// # Container attributes
///
/// - `#[table("name")]` — table name (default: `snake_case` struct name + `s`).
/// - `#[entity(crate = "path")]` — path to `rustali_orm` (default `::rustali::orm`).
///
/// # Field attributes
///
/// - `#[key]` / `#[key(auto = false)]` — primary key; integer keys auto-increment
///   unless told otherwise. Without any `#[key]`, a field named `id` is the key.
/// - `#[column(name = "col", unique, max_len = N)]` — column options.
/// - `#[skip]` — not stored; must implement `Default`.
///
/// # Conventions
///
/// `Option<T>` fields are nullable. Fields named `created_at` / `updated_at` are
/// maintained automatically on insert and update.
///
/// # Generated API
///
/// `Todo::query()`, `Todo::find(&db, id)`, `Todo::find_or_404(&db, id)`,
/// `Todo::all(&db)`, `Todo::count(&db)`, `todo.insert(&db)`, `todo.update(&db)`,
/// `todo.delete(&db)`, typed column constants (`Todo::TITLE`), plus the
/// `rustali_orm::Entity` implementation and a hidden `todo_entity` module with the
/// SeaORM types.
#[proc_macro_derive(Entity, attributes(table, entity, key, column, skip))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    entity::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Attribute routing: declare routes on the handlers themselves.
///
/// ```ignore
/// use rustali::prelude::*;
///
/// pub struct PostController;
///
/// #[controller("/posts")]
/// #[middleware(Throttle::per_minute(60))]
/// impl PostController {
///     #[get("/")]
///     async fn index(db: Db) -> Result<Json<Vec<Post>>> { /* ... */ }
///
///     #[get("/{id}", name = "posts.show")]
///     async fn show(db: Db, Path(id): Path<i64>) -> Result<Json<Post>> { /* ... */ }
///
///     #[post("/")]
///     #[authorize(role = "editor")]
///     async fn store(db: Db, Json(input): Json<NewPost>) -> Result<Json<Post>> { /* ... */ }
/// }
/// ```
///
/// Inside the `impl`:
///
/// - `#[get("/path")]`, `#[post]`, `#[put]`, `#[patch]`, `#[delete]`, `#[head]`,
///   `#[options]` — one per handler; optional `name = "..."`. Paths use axum syntax
///   (`{id}`) and are joined with the controller prefix.
/// - `#[middleware(expr)]` — a tower layer for one handler (on a method) or for the
///   whole controller (on the `impl`). Method layers run inside controller layers.
/// - `#[authorize("ability")]` / `#[authorize(role = "name")]` — require an ability
///   through the application's `Gate`.
///
/// Handlers are `async fn` without `self`; the type is a stateless namespace. The
/// macro implements `rustali::router::Controller` (giving `PostController::router()`
/// and `::routes()`) and registers the controller for `App::builder().controllers()`.
///
/// Options: `#[controller("/prefix", crate = "rustali_core")]` changes the framework
/// path used by generated code.
#[proc_macro_attribute]
pub fn controller(attr: TokenStream, item: TokenStream) -> TokenStream {
    controller::expand(attr.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
