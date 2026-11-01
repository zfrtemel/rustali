# Rustali

> Laravel's developer experience, .NET-grade engineering, Rust's safety and speed.
> A batteries-included full-stack web framework — **pre-alpha, under construction.**

```bash
# Scaffold and run a new app (once 0.1.0 is published):
cargo install rustali-cli
rustali new blog && cd blog
cargo run -- migrate
cargo run                 # http://127.0.0.1:8000
rustali make:controller Post   # generate a controller
```

```rust
use rustali::prelude::*;

pub struct PostController;

#[controller("/posts")]
#[middleware(Throttle::per_minute(60))]
impl PostController {
    #[get("/{id}", name = "posts.show")]
    async fn show(db: Db, Path(id): Path<i64>) -> Result<Json<Post>> {
        Ok(Json(Post::find_or_404(&db, id).await?))
    }

    #[post("/")]
    #[authorize(role = "editor")]
    async fn store(db: Db, Json(input): Json<NewPost>) -> Result<Json<Post>> {
        Ok(Json(Post::from(input).insert(&db).await?))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    App::builder()
        .provider(DatabaseProvider)
        .controllers()
        .run()
        .await
}
```

## What Rustali is

One dependency that gives you HTTP routing, an ORM with **code-first diff
migrations**, type-safe server-rendered views **and** a JSON API from the same
controllers, authentication that ships ready to use, background jobs, mail, cache,
storage and a CLI (`rustali new`, `rustali make:controller`, `rustali migrate:diff`).

Five things set it apart from existing Rust frameworks:

| | |
|---|---|
| **Attribute routing** | `#[controller("/posts")]` + `#[get("/{id}")]` — routes live on the handler, no separate route file; `rustali routes` lists them all. |
| **POCO entities** | One plain struct with `#[derive(Entity)]`; the macro generates the SeaORM plumbing you never see. |
| **Code-first migrations** | Change the struct, run `rustali migrate:diff`, review the generated migration. EF Core style. |
| **Layered filter pipeline** | Global → controller → action middleware, `#[authorize]`, exception filters — all tower layers, all explicit. |
| **Type-safe views** | Askama templates checked at compile time by default; Tera available when you want runtime flexibility. |

Built on [axum](https://github.com/tokio-rs/axum), [tokio](https://tokio.rs),
[SeaORM](https://www.sea-ql.org/SeaORM/) and [askama](https://github.com/askama-rs/askama).
"Explicit over magic": macros generate code you can read with `cargo expand`; there is
no runtime reflection.

## Status

Pre-alpha. The roadmap, architecture and coding standard live in
[VISION.md](VISION.md) and [docs/adr](docs/adr). Nothing is published to crates.io yet.

What works today (run from a checkout):

```bash
cargo run -p hello                        # SSR page + JSON endpoint, negotiated errors
cargo run -p todo-api -- migrate          # SQLite schema via sea-orm-migration
cargo run -p todo-api                     # CRUD JSON API on a #[derive(Entity)] struct
cargo run -p todo-api -- routes           # list controller routes (Laravel's route:list)
cargo run -p blog -- migrate && cargo run -p blog
#                                           server-rendered CRUD: Askama views,
#                                           #[derive(Validate)], flash messages, CSRF
cargo run -p auth-app -- migrate && cargo run -p auth-app
#                                           register / login / logout / protected page:
#                                           argon2 hashing, sessions, CSRF, validation
cargo run -p todo-api -- migrate:diff "add a field"
#                                           code-first: change an entity, diff against
#                                           the snapshot, get a reviewed migration
cargo test -p blog                        # drive the whole app in-process with TestClient
```

Ecosystem packages (all behind default-on features): `cache` (`remember()`, memory +
Redis), `mail` (`Mailable` + SMTP/log/memory), `queue` (`Job` + worker + retries),
plus a `test` kit (`TestClient` with fluent assertions).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security reports: [SECURITY.md](SECURITY.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
