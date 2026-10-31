//! The smallest Rustali application.
//!
//! Run with `cargo run -p hello`, then try:
//!
//! - <http://127.0.0.1:8000/> — server-rendered HTML
//! - <http://127.0.0.1:8000/api/ping> — JSON
//! - <http://127.0.0.1:8000/users/42> and `/users/0` — typed path parameter, 404 as JSON or HTML
//! - <http://127.0.0.1:8000/boom> — an internal error; detail is shown because `debug = true`

use rustali::{
    http::{Router, routing::get},
    prelude::*,
};
use serde::Serialize;

#[tokio::main]
async fn main() -> Result<()> {
    // The example lives inside the workspace, so locate its config directory from
    // the crate root rather than the current working directory.
    let config = Config::load_from(concat!(env!("CARGO_MANIFEST_DIR"), "/config"))?;

    App::builder().config(config).routes(routes()).serve().await
}

fn routes() -> Router<AppContext> {
    Router::new()
        .route("/", get(home))
        .route("/api/ping", get(ping))
        .route("/users/{id}", get(show_user))
        .route("/boom", get(boom))
}

async fn home(ctx: Ctx) -> Html<String> {
    let name = &ctx.config().app.name;
    Html(format!(
        "<h1>{name}</h1><p>It works. Try <a href=\"/api/ping\">/api/ping</a>.</p>"
    ))
}

#[derive(Serialize)]
struct Pong {
    ok: bool,
    app: String,
}

async fn ping(ctx: Ctx) -> Json<Pong> {
    Json(Pong {
        ok: true,
        app: ctx.config().app.name.clone(),
    })
}

#[derive(Serialize)]
struct User {
    id: u64,
    name: String,
}

async fn show_user(Path(id): Path<u64>) -> Result<Json<User>> {
    if id == 0 {
        return Err(Error::NotFound);
    }
    Ok(Json(User {
        id,
        name: format!("user-{id}"),
    }))
}

async fn boom() -> Result<()> {
    Err(Error::internal(std::io::Error::other("database exploded")))
}
