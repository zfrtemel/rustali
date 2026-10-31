//! HTTP controllers. Routes live on the handlers; `App::builder().controllers()`
//! mounts every `#[controller]` in the binary.

use rustali::{http::StatusCode, prelude::*};
use serde::Deserialize;

use crate::models::Todo;

#[derive(Deserialize)]
pub struct Paging {
    page: Option<u64>,
    per_page: Option<u64>,
}

#[derive(Deserialize)]
pub struct CreateTodo {
    title: String,
}

#[derive(Deserialize)]
pub struct UpdateTodo {
    title: Option<String>,
    done: Option<bool>,
}

pub struct TodoController;

#[controller("/api/todos")]
#[middleware(Throttle::per_minute(600))]
impl TodoController {
    #[get("/", name = "todos.index")]
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

    #[post("/", name = "todos.store")]
    async fn store(db: Db, Json(input): Json<CreateTodo>) -> Result<(StatusCode, Json<Todo>)> {
        let title = input.title.trim();
        if title.is_empty() {
            return Err(Error::bad_request("title must not be empty"));
        }
        let todo = Todo::new(title).insert(&db).await?;
        Ok((StatusCode::CREATED, Json(todo)))
    }

    #[get("/{id}", name = "todos.show")]
    async fn show(db: Db, Path(id): Path<i64>) -> Result<Json<Todo>> {
        Ok(Json(Todo::find_or_404(&db, id).await?))
    }

    #[patch("/{id}", name = "todos.update")]
    async fn update(
        db: Db,
        Path(id): Path<i64>,
        Json(input): Json<UpdateTodo>,
    ) -> Result<Json<Todo>> {
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

    #[delete("/{id}", name = "todos.destroy")]
    async fn destroy(db: Db, Path(id): Path<i64>) -> Result<StatusCode> {
        Todo::find_or_404(&db, id).await?.delete(&db).await?;
        Ok(StatusCode::NO_CONTENT)
    }
}
