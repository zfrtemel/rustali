//! `#[derive(Entity)]` end to end against an in-memory SQLite database.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rustali_core::Error;
use rustali_macros::Entity;
use rustali_orm::{
    ColumnMeta, Entity as _,
    chrono::{DateTime, Utc},
    sea_orm::{
        ColumnTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend, QueryFilter,
        QueryOrder, Schema,
    },
};

#[derive(Entity, Debug, Clone, PartialEq)]
#[entity(crate = "rustali_orm")]
#[table("todos")]
struct Todo {
    id: i64,
    #[column(max_len = 200)]
    title: String,
    done: bool,
    note: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    #[skip]
    cached: u8,
}

fn todo(title: &str, done: bool) -> Todo {
    Todo {
        id: 0,
        title: title.into(),
        done,
        note: None,
        created_at: DateTime::<Utc>::MIN_UTC,
        updated_at: DateTime::<Utc>::MIN_UTC,
        cached: 42,
    }
}

async fn db() -> DatabaseConnection {
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite");
    let schema = Schema::new(DbBackend::Sqlite);
    let stmt = schema.create_table_from_entity(todo_entity::Entity);
    db.execute(&stmt).await.expect("create table");
    db
}

#[test]
fn metadata_describes_the_table() {
    assert_eq!(Todo::TABLE, "todos");
    let names: Vec<&str> = Todo::COLUMNS.iter().map(|c| c.name).collect();
    assert_eq!(
        names,
        ["id", "title", "done", "note", "created_at", "updated_at"]
    );
    let id = Todo::COLUMNS[0];
    assert!(id.primary_key && id.auto_increment && !id.nullable);
    let title: &ColumnMeta = &Todo::COLUMNS[1];
    assert_eq!(title.max_len, Some(200));
    assert!(Todo::COLUMNS[3].nullable);
    assert_eq!(Todo::COLUMNS[3].rust_type, "Option<String>");
}

#[tokio::test]
async fn insert_assigns_key_and_timestamps() {
    let db = db().await;
    let before = Utc::now();
    let saved = todo("write tests", false).insert(&db).await.unwrap();
    assert_eq!(saved.id, 1);
    assert!(
        saved.created_at >= before,
        "created_at is set by the framework"
    );
    assert_eq!(saved.created_at, saved.updated_at);
    assert_eq!(saved.cached, 0, "skipped fields come back as Default");
}

#[tokio::test]
async fn find_and_find_or_404() {
    let db = db().await;
    let saved = todo("a", false).insert(&db).await.unwrap();
    assert_eq!(
        Todo::find(&db, saved.id).await.unwrap().as_ref(),
        Some(&saved)
    );
    assert!(Todo::find(&db, 999).await.unwrap().is_none());
    assert!(matches!(
        Todo::find_or_404(&db, 999).await,
        Err(Error::NotFound)
    ));
}

#[tokio::test]
async fn update_writes_columns_and_bumps_updated_at() {
    let db = db().await;
    let mut saved = todo("draft", false).insert(&db).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    saved.title = "final".into();
    saved.done = true;
    let updated = saved.clone().update(&db).await.unwrap();
    assert_eq!(updated.title, "final");
    assert!(updated.done);
    assert_eq!(
        updated.created_at, saved.created_at,
        "created_at never changes"
    );
    assert!(updated.updated_at > saved.updated_at);
}

#[tokio::test]
async fn queries_fetch_plain_structs() {
    let db = db().await;
    for i in 0..5 {
        todo(&format!("task {i}"), i % 2 == 0)
            .insert(&db)
            .await
            .unwrap();
    }
    assert_eq!(Todo::count(&db).await.unwrap(), 5);
    assert_eq!(Todo::all(&db).await.unwrap().len(), 5);

    let done: Vec<Todo> = Todo::query()
        .filter(Todo::DONE.eq(true))
        .order_by_desc(Todo::ID)
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(
        done.iter().map(|t| t.title.as_str()).collect::<Vec<_>>(),
        ["task 4", "task 2", "task 0"]
    );

    let first = Todo::query()
        .filter(Todo::TITLE.eq("task 1"))
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(first.map(|t| t.done), Some(false));

    let page = Todo::query()
        .order_by_asc(Todo::ID)
        .fetch_page(&db, 2, 2)
        .await
        .unwrap();
    assert_eq!(page.total, 5);
    assert_eq!(page.last_page, 3);
    assert_eq!(page.items.iter().map(|t| t.id).collect::<Vec<_>>(), [3, 4]);
    assert!(page.has_next() && page.has_prev());
}

#[tokio::test]
async fn delete_removes_the_row() {
    let db = db().await;
    let saved = todo("gone", false).insert(&db).await.unwrap();
    saved.delete(&db).await.unwrap();
    assert_eq!(Todo::count(&db).await.unwrap(), 0);
}
