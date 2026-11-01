//! Soft delete end to end: with the global switch on, `delete` stamps `deleted_at` and
//! `query`/`find` hide trashed rows; with it off, `delete` removes the row.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rustali_macros::Entity;
use rustali_orm::{
    chrono::{DateTime, Utc},
    sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, EntityTrait, Schema},
    soft_delete,
};

#[derive(Entity, Debug, Clone, PartialEq)]
#[entity(crate = "rustali_orm")]
#[table("notes")]
struct Note {
    id: i64,
    body: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

fn note(body: &str) -> Note {
    Note {
        id: 0,
        body: body.into(),
        created_at: DateTime::<Utc>::MIN_UTC,
        updated_at: DateTime::<Utc>::MIN_UTC,
        deleted_at: None,
    }
}

async fn fresh_db() -> DatabaseConnection {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let schema = Schema::new(DbBackend::Sqlite);
    db.execute(&schema.create_table_from_entity(note_entity::Entity))
        .await
        .unwrap();
    db
}

// These two tests flip a process-global switch, so they must not run concurrently with
// each other. Keep them in one test function to serialize the state changes.
#[tokio::test]
async fn soft_delete_on_and_off() {
    // --- soft delete ON ---
    soft_delete::set_enabled(true);
    let db = fresh_db().await;
    let kept = note("keep").insert(&db).await.unwrap();
    let gone = note("trash me").insert(&db).await.unwrap();

    gone.clone().delete(&db).await.unwrap();

    // the row still exists in the table...
    assert_eq!(
        note_entity::Entity::find().all(&db).await.unwrap().len(),
        2,
        "soft delete keeps the row"
    );
    // ...but is hidden from the entity's query/find
    assert_eq!(
        Note::count(&db).await.unwrap(),
        1,
        "trashed row hidden from count"
    );
    assert_eq!(
        Note::all(&db).await.unwrap().len(),
        1,
        "trashed row hidden from all"
    );
    assert!(
        Note::find(&db, gone.id).await.unwrap().is_none(),
        "find hides trashed"
    );
    assert!(
        Note::find(&db, kept.id).await.unwrap().is_some(),
        "live row still found"
    );

    // and deleted_at was stamped
    let raw = note_entity::Entity::find_by_id(gone.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(raw.deleted_at.is_some(), "deleted_at stamped");

    // --- soft delete OFF ---
    soft_delete::set_enabled(false);
    let db2 = fresh_db().await;
    let n = note("hard").insert(&db2).await.unwrap();
    n.delete(&db2).await.unwrap();
    assert_eq!(
        note_entity::Entity::find().all(&db2).await.unwrap().len(),
        0,
        "hard delete removes the row"
    );

    soft_delete::set_enabled(false); // reset for any other tests in this binary
}
