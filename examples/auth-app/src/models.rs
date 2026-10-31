//! The `User` entity and its `Authenticatable` + `UserStore` wiring.

use rustali::{
    orm::chrono::{DateTime, Utc},
    prelude::*,
};
use serde::Serialize;

#[derive(Entity, Debug, Clone, Serialize)]
#[table("users")]
pub struct User {
    pub id: i64,
    pub name: String,
    #[column(unique)]
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Build an unsaved user with an already-hashed password.
    pub fn new(name: &str, email: &str, password_hash: String) -> Self {
        Self {
            id: 0,
            name: name.to_owned(),
            email: email.to_owned(),
            password_hash,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Authenticatable for User {
    fn auth_id(&self) -> String {
        self.id.to_string()
    }

    fn password_hash(&self) -> &str {
        &self.password_hash
    }
}

/// Loads users from the database for the auth guards.
pub struct Users;

impl UserStore for Users {
    type User = User;

    fn find_by_id<'a>(&'a self, ctx: &'a AppContext, id: &'a str) -> StoreFuture<'a, Option<User>> {
        Box::pin(async move {
            let Ok(id) = id.parse::<i64>() else {
                return Ok(None);
            };
            User::find(&*ctx.db()?, id).await
        })
    }

    fn find_by_login<'a>(
        &'a self,
        ctx: &'a AppContext,
        email: &'a str,
    ) -> StoreFuture<'a, Option<User>> {
        Box::pin(async move {
            User::query()
                .filter(User::EMAIL.eq(email))
                .fetch_one(&*ctx.db()?)
                .await
        })
    }
}
