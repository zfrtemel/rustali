# Rustali

> Laravel's developer experience, .NET-grade engineering, Rust's safety and speed.

Rustali is a batteries-included, full-stack web framework for Rust. You add **one**
dependency and get HTTP routing, an ORM with code-first migrations, type-safe views
*and* a JSON API from the same controllers, authentication that ships ready to use,
background jobs, mail, cache, and a CLI — all designed to fit together.

```rust
use rustali::prelude::*;

pub struct PostController;

#[controller("/posts")]
impl PostController {
    #[get("/{id}", name = "posts.show")]
    async fn show(db: Db, Path(id): Path<i64>) -> Result<Json<Post>> {
        Ok(Json(Post::find_or_404(&db, id).await?))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    App::builder().provider(DatabaseProvider).controllers().run().await
}
```

## Why Rustali?

Rust had excellent *parts* — [axum] for HTTP, [SeaORM] for data, [askama] for
templates — but assembling them into a coherent, convention-driven whole was left to
you. Rustali is that assembly, with a point of view borrowed from two mature
ecosystems:

- **From Laravel:** developer experience. It works when you open it. Generators,
  conventions, flash messages, validation that redirects back with old input, an
  `artisan`-style console.
- **From .NET Core:** engineering discipline. Attribute routing on the handler itself,
  a layered filter pipeline, explicit dependency injection, and — the hard one —
  **code-first diff migrations**: change your entity, run `migrate:diff`, review the
  generated migration.

And the Rust rule that governs both: **explicit over magic.** Macros generate code you
can read with `cargo expand`; there is no runtime reflection.

## The five differentiators

| | |
|---|---|
| **Attribute routing** | Routes live on the handler; `rustali routes` lists them all. |
| **POCO entities** | One plain struct with `#[derive(Entity)]`; the SeaORM plumbing is generated and hidden. |
| **Code-first migrations** | `migrate:diff` turns entity changes into reviewed migrations. |
| **Layered filter pipeline** | Global → controller → action middleware, `#[authorize]`, exception filters — all tower layers. |
| **Type-safe views** | Askama templates checked at compile time by default; Tera available for runtime flexibility. |

[axum]: https://github.com/tokio-rs/axum
[SeaORM]: https://www.sea-ql.org/SeaORM/
[askama]: https://github.com/askama-rs/askama

> **Status:** pre-alpha. The API will change before 1.0. This book tracks `main`.
