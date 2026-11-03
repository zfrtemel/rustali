//! Domain models: plain structs marked with `#[entity]`.

use rustali::{
    orm::chrono::{DateTime, Utc},
    prelude::*,
};

/// One todo item. The struct *is* the API representation; SeaORM plumbing is
/// generated into a hidden `todo_entity` module.
#[entity]
#[table("todos")]
pub struct Todo {
    pub id: i64,
    #[column(max_len = 200)]
    pub title: String,
    pub done: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Todo {
    /// A new, unsaved todo. Key and timestamps are filled in by `insert`.
    pub fn new(title: &str) -> Self {
        Self {
            id: 0,
            title: title.to_owned(),
            done: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
