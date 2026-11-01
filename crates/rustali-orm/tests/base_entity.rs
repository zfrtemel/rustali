//! The `BaseEntity` / `SoftDelete` accessors a scaffolded model implements, exercised the
//! way `rustali make:model` writes them.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rustali_macros::Entity;
use rustali_orm::{
    BaseEntity, SoftDelete,
    chrono::{DateTime, Utc},
};

#[derive(Entity, Debug, Clone, PartialEq)]
#[entity(crate = "rustali_orm")]
#[table("articles")]
struct Article {
    id: i64,
    title: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

impl BaseEntity for Article {
    type Id = i64;
    fn id(&self) -> Self::Id {
        self.id
    }
    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

impl SoftDelete for Article {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
}

fn article(deleted: bool) -> Article {
    let now = Utc::now();
    Article {
        id: 7,
        title: "hello".into(),
        created_at: now,
        updated_at: now,
        deleted_at: deleted.then_some(now),
    }
}

#[test]
fn base_accessors_return_the_fields() {
    let a = article(false);
    assert_eq!(a.id(), 7);
    assert_eq!(a.created_at(), a.updated_at());
}

#[test]
fn soft_delete_reports_trashed_state() {
    assert!(!article(false).is_trashed());
    assert!(article(true).is_trashed());
    assert!(article(true).deleted_at().is_some());
}

/// Generic code can work over any base model without naming the concrete type.
#[test]
fn works_through_the_trait() {
    fn describe<T: BaseEntity<Id = i64>>(row: &T) -> i64 {
        row.id()
    }
    assert_eq!(describe(&article(false)), 7);
}
