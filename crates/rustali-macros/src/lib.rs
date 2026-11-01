//! Procedural macros for the Rustali web framework.
//!
//! This crate is an implementation detail re-exported by `rustali`; it must only
//! depend on the proc-macro toolkit. Generated code refers to the framework through a
//! configurable path (default `::rustali`) so it works whether the user depends on
//! the meta crate or on the individual crates.

mod controller;
mod entity;
mod validate;

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
#[proc_macro_derive(
    Entity,
    attributes(
        table,
        entity,
        key,
        column,
        skip,
        has_many,
        has_one,
        belongs_to,
        many_to_many
    )
)]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    entity::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Declarative input validation.
///
/// ```ignore
/// #[derive(Deserialize, Validate)]
/// struct CreatePost {
///     #[rule(required, length(min = 3, max = 200))]
///     title: String,
///     #[rule(email)]
///     author_email: Option<String>,
///     #[rule(range(min = 1, max = 5))]
///     rating: u8,
///     #[rule(one_of("draft", "published"))]
///     status: String,
///     #[rule(confirmed)]
///     password: String,
///     password_confirmation: String,
///     #[rule(custom = "no_profanity")]
///     body: String,
///     #[rule(nested)]
///     address: Address,
/// }
///
/// fn no_profanity(body: &String) -> Result<(), String> { /* ... */ }
/// ```
///
/// Rules: `required`, `length(min, max)`, `range(min, max)`, `email`, `url`,
/// `alpha_num`, `one_of("a", "b")`, `confirmed` (compares with
/// `<field>_confirmation`), `custom = "fn_path"` (`fn(&T) -> Result<(), impl Display>`),
/// `nested` (field implements `Validate`). On `Option<T>` fields every rule except
/// `required` is skipped when the value is `None`. All failures are collected.
///
/// Option: `#[validate(crate = "rustali_validate")]`.
#[proc_macro_derive(Validate, attributes(rule, validate))]
pub fn derive_validate(input: TokenStream) -> TokenStream {
    validate::expand(input.into())
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
