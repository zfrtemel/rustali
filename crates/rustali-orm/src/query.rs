//! [`Query<E>`]: a SeaORM `Select` that knows which struct its rows become.

use rustali_core::Result;
use sea_orm::{
    ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait,
    Select, sea_query::SelectStatement,
};

use crate::{Entity, Page, error::map_db_err};

/// A typed query builder for an [`Entity`].
///
/// Wraps `Select<E::Sea>` and forwards SeaORM's [`QueryFilter`], [`QueryOrder`] and
/// [`QuerySelect`] so every familiar builder method works, while the terminal methods
/// return plain structs instead of SeaORM models:
///
/// ```ignore
/// let recent = Post::query()
///     .filter(Post::PUBLISHED.eq(true))
///     .order_by_desc(Post::CREATED_AT)
///     .limit(10)
///     .fetch_all(&db)
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct Query<E: Entity> {
    select: Select<E::Sea>,
}

impl<E: Entity> Query<E> {
    /// Start from `SELECT * FROM table`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            select: E::Sea::find(),
        }
    }

    /// Wrap an existing SeaORM select.
    #[must_use]
    pub const fn from_select(select: Select<E::Sea>) -> Self {
        Self { select }
    }

    /// Hand the raw SeaORM select back for APIs this wrapper does not cover.
    #[must_use]
    pub fn into_select(self) -> Select<E::Sea> {
        self.select
    }

    /// All matching rows.
    pub async fn fetch_all<C: ConnectionTrait>(self, db: &C) -> Result<Vec<E>> {
        let models = self.select.all(db).await.map_err(map_db_err)?;
        Ok(models.into_iter().map(Into::into).collect())
    }

    /// The first matching row, if any.
    pub async fn fetch_one<C: ConnectionTrait>(self, db: &C) -> Result<Option<E>> {
        let model = self.select.one(db).await.map_err(map_db_err)?;
        Ok(model.map(Into::into))
    }

    /// The first matching row or `Error::NotFound`.
    pub async fn fetch_one_or_404<C: ConnectionTrait>(self, db: &C) -> Result<E> {
        self.fetch_one(db)
            .await?
            .ok_or(rustali_core::Error::NotFound)
    }

    /// Number of matching rows.
    pub async fn count<C: ConnectionTrait>(self, db: &C) -> Result<u64> {
        self.select.count(db).await.map_err(map_db_err)
    }

    /// `true` when at least one row matches.
    pub async fn exists<C: ConnectionTrait>(self, db: &C) -> Result<bool> {
        Ok(self.limit(1).count(db).await? > 0)
    }

    /// A 1-based page of results with totals.
    ///
    /// `page` is clamped to `>= 1` and `per_page` to `>= 1`, so out-of-range input from a
    /// query string can never panic or divide by zero.
    pub async fn paginate<C: ConnectionTrait>(
        self,
        db: &C,
        page: u64,
        per_page: u64,
    ) -> Result<Page<E>> {
        let page = page.max(1);
        let per_page = per_page.max(1);
        let paginator = self.select.paginate(db, per_page);
        let total = paginator.num_items().await.map_err(map_db_err)?;
        let models = paginator.fetch_page(page - 1).await.map_err(map_db_err)?;
        Ok(Page::new(
            models.into_iter().map(Into::into).collect(),
            page,
            per_page,
            total,
        ))
    }

    /// A 1-based page of results with totals.
    ///
    /// Alias for [`paginate`](Self::paginate), kept for the `fetch_*` naming of the other
    /// terminal methods.
    pub async fn fetch_page<C: ConnectionTrait>(
        self,
        db: &C,
        page: u64,
        per_page: u64,
    ) -> Result<Page<E>> {
        self.paginate(db, page, per_page).await
    }
}

impl<E: Entity> Default for Query<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Entity> QueryTrait for Query<E> {
    type QueryStatement = SelectStatement;

    fn query(&mut self) -> &mut SelectStatement {
        QueryTrait::query(&mut self.select)
    }

    fn as_query(&self) -> &SelectStatement {
        self.select.as_query()
    }

    fn into_query(self) -> SelectStatement {
        self.select.into_query()
    }
}

impl<E: Entity> QueryFilter for Query<E> {
    type QueryStatement = SelectStatement;

    fn query(&mut self) -> &mut SelectStatement {
        QueryTrait::query(&mut self.select)
    }
}

impl<E: Entity> QueryOrder for Query<E> {
    type QueryStatement = SelectStatement;

    fn query(&mut self) -> &mut SelectStatement {
        QueryTrait::query(&mut self.select)
    }
}

impl<E: Entity> QuerySelect for Query<E> {
    type QueryStatement = SelectStatement;

    fn query(&mut self) -> &mut SelectStatement {
        QueryTrait::query(&mut self.select)
    }
}
