//! The `Post` entity.

use rustali::{
    orm::chrono::{DateTime, Utc},
    prelude::*,
};

#[entity]
#[table("posts")]
pub struct Post {
    pub id: i64,
    #[column(max_len = 200)]
    pub title: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Post {
    pub fn new(title: &str, body: &str) -> Self {
        Self {
            id: 0,
            title: title.to_owned(),
            body: body.to_owned(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
