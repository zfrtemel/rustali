//! The [`Db`] extractor.

use std::{
    future::{Future, ready},
    ops::Deref,
};

use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use rustali_core::{AppContext, Error};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, ExecResult, QueryResult, Statement,
    prelude::async_trait::async_trait,
};

use crate::DbExt;

/// A handle to the connection pool for use in handlers.
///
/// Cloning the underlying [`DatabaseConnection`] is cheap (it is a pool handle).
/// `Db` implements [`ConnectionTrait`], so `&db` goes straight into any SeaORM or
/// Rustali query API:
///
/// ```ignore
/// async fn index(db: Db) -> Result<Json<Vec<Todo>>> {
///     Ok(Json(Todo::all(&db).await?))
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Db(pub DatabaseConnection);

impl<S> FromRequestParts<S> for Db
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    fn from_request_parts(
        _parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        ready(
            AppContext::from_ref(state)
                .db()
                .map(|database| Self(database.conn().clone())),
        )
    }
}

impl Deref for Db {
    type Target = DatabaseConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl ConnectionTrait for Db {
    fn get_database_backend(&self) -> DbBackend {
        self.0.get_database_backend()
    }

    async fn execute_raw(&self, stmt: Statement) -> Result<ExecResult, DbErr> {
        self.0.execute_raw(stmt).await
    }

    async fn execute_unprepared(&self, sql: &str) -> Result<ExecResult, DbErr> {
        self.0.execute_unprepared(sql).await
    }

    async fn query_one_raw(&self, stmt: Statement) -> Result<Option<QueryResult>, DbErr> {
        self.0.query_one_raw(stmt).await
    }

    async fn query_all_raw(&self, stmt: Statement) -> Result<Vec<QueryResult>, DbErr> {
        self.0.query_all_raw(stmt).await
    }

    fn support_returning(&self) -> bool {
        self.0.support_returning()
    }
}
