//! A JSON API backed by SQLite: migrations, a `#[derive(Entity)]` model and CRUD.
//!
//! ```text
//! cargo run -p todo-api -- migrate          # create the schema
//! cargo run -p todo-api -- migrate:status
//! cargo run -p todo-api                      # serve on http://127.0.0.1:8000
//!
//! curl -X POST localhost:8000/api/todos -H 'content-type: application/json' -d '{"title":"ship it"}'
//! curl localhost:8000/api/todos?page=1&per_page=20
//! curl -X PATCH localhost:8000/api/todos/1 -H 'content-type: application/json' -d '{"done":true}'
//! curl -X DELETE localhost:8000/api/todos/1
//! ```

mod migrations;
mod models;

use rustali::{
    http::{Router, StatusCode, routing::get},
    prelude::*,
};
use serde::Deserialize;

use crate::models::Todo;

#[tokio::main]
async fn main() -> Result<()> {
    // Examples live inside the workspace; run from their own directory so relative
    // paths in config/database.toml resolve the same way a real app's would.
    std::env::set_current_dir(env!("CARGO_MANIFEST_DIR"))?;

    App::builder()
        .provider(DatabaseProvider)
        .migrations::<migrations::Migrator>()
        .routes(routes())
        .run()
        .await
}

fn routes() -> Router<AppContext> {
    Router::new()
        .route("/api/todos", get(index).post(store))
        .route("/api/todos/{id}", get(show).patch(update).delete(destroy))
}

#[derive(Deserialize)]
struct Paging {
    page: Option<u64>,
    per_page: Option<u64>,
}

async fn index(db: Db, Query(paging): Query<Paging>) -> Result<Json<Page<Todo>>> {
    let page = Todo::query()
        .order_by_desc(Todo::ID)
        .fetch_page(
            &db,
            paging.page.unwrap_or(1),
            paging.per_page.unwrap_or(20).min(100),
        )
        .await?;
    Ok(Json(page))
}

#[derive(Deserialize)]
struct CreateTodo {
    title: String,
}

async fn store(db: Db, Json(input): Json<CreateTodo>) -> Result<(StatusCode, Json<Todo>)> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(Error::bad_request("title must not be empty"));
    }
    let todo = Todo::new(title).insert(&db).await?;
    Ok((StatusCode::CREATED, Json(todo)))
}

async fn show(db: Db, Path(id): Path<i64>) -> Result<Json<Todo>> {
    Ok(Json(Todo::find_or_404(&db, id).await?))
}

#[derive(Deserialize)]
struct UpdateTodo {
    title: Option<String>,
    done: Option<bool>,
}

async fn update(db: Db, Path(id): Path<i64>, Json(input): Json<UpdateTodo>) -> Result<Json<Todo>> {
    let mut todo = Todo::find_or_404(&db, id).await?;
    if let Some(title) = input.title {
        if title.trim().is_empty() {
            return Err(Error::bad_request("title must not be empty"));
        }
        todo.title = title.trim().to_owned();
    }
    if let Some(done) = input.done {
        todo.done = done;
    }
    Ok(Json(todo.update(&db).await?))
}

async fn destroy(db: Db, Path(id): Path<i64>) -> Result<StatusCode> {
    Todo::find_or_404(&db, id).await?.delete(&db).await?;
    Ok(StatusCode::NO_CONTENT)
}
