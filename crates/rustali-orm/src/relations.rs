//! Relation loading helpers used by `#[derive(Entity)]`-generated methods.
//!
//! Rust cannot do Laravel-style implicit lazy loading — a field access can neither be
//! `async` nor run hidden I/O — so every relation load is explicit and `.await`ed. Two
//! shapes are offered:
//!
//! - **Lazy (on demand):** `post.comments(&db).await?` loads one parent's related rows.
//! - **Eager (batch, avoids N+1):** `Post::with_comments(posts, &db).await?` loads all
//!   the children for a set of parents in one extra query, pairing them up.
//!
//! The generated methods are thin wrappers over the functions here, which in turn use
//! SeaORM's `find_related` (lazy) and `LoaderTrait` (eager). Many-to-many relations
//! route through a junction entity: lazily they still go through `find_related` (the
//! generated `Related` impl declares the junction via `via()`), and eagerly through
//! [`eager_many_to_many`], which uses SeaORM's `load_many_to_many`.

use rustali_core::Result;
use sea_orm::{ConnectionTrait, EntityTrait, LoaderTrait, ModelTrait, Related};

use crate::{Entity, error::map_db_err};

/// Lazily load the *many* side of a relation for one parent: `Vec<Child>`.
///
/// `parent` is the parent entity; `R` is the child entity. Used by generated `has_many`
/// accessors.
pub async fn load_many<P, R, C>(parent: &P, db: &C) -> Result<Vec<R>>
where
    P: Entity + Clone,
    R: Entity,
    C: ConnectionTrait,
    P::Sea: Related<R::Sea>,
{
    let model: <P::Sea as EntityTrait>::Model = parent.clone().into();
    let related = model
        .find_related(R::Sea::default())
        .all(db)
        .await
        .map_err(map_db_err)?;
    Ok(related.into_iter().map(Into::into).collect())
}

/// Lazily load the *one* side of a relation for one parent: `Option<Child>`.
///
/// Used by generated `belongs_to` / `has_one` accessors.
pub async fn load_one<P, R, C>(parent: &P, db: &C) -> Result<Option<R>>
where
    P: Entity + Clone,
    R: Entity,
    C: ConnectionTrait,
    P::Sea: Related<R::Sea>,
{
    let model: <P::Sea as EntityTrait>::Model = parent.clone().into();
    let related = model
        .find_related(R::Sea::default())
        .one(db)
        .await
        .map_err(map_db_err)?;
    Ok(related.map(Into::into))
}

/// Eagerly load the *many* side for a set of parents in one extra query, pairing each
/// parent with its children. Used by generated `with_*` methods on `Query`.
pub async fn eager_many<P, R, C>(parents: Vec<P>, db: &C) -> Result<Vec<(P, Vec<R>)>>
where
    P: Entity + Clone,
    R: Entity,
    C: ConnectionTrait,
    P::Sea: Related<R::Sea>,
{
    let models: Vec<<P::Sea as EntityTrait>::Model> =
        parents.iter().map(|p| p.clone().into()).collect();
    let children = models
        .load_many(R::Sea::default(), db)
        .await
        .map_err(map_db_err)?;
    Ok(parents
        .into_iter()
        .zip(children)
        .map(|(parent, kids)| (parent, kids.into_iter().map(Into::into).collect()))
        .collect())
}

/// Eagerly load the *one* side for a set of parents in one extra query.
pub async fn eager_one<P, R, C>(parents: Vec<P>, db: &C) -> Result<Vec<(P, Option<R>)>>
where
    P: Entity + Clone,
    R: Entity,
    C: ConnectionTrait,
    P::Sea: Related<R::Sea>,
{
    let models: Vec<<P::Sea as EntityTrait>::Model> =
        parents.iter().map(|p| p.clone().into()).collect();
    let related = models
        .load_one(R::Sea::default(), db)
        .await
        .map_err(map_db_err)?;
    Ok(parents
        .into_iter()
        .zip(related)
        .map(|(p, r)| (p, r.map(Into::into)))
        .collect())
}

/// Eagerly load a many-to-many relation for a set of parents, routing through the
/// junction entity `V` with two extra queries (avoids N+1). Used by generated `with_*`
/// methods for `many_to_many` relations.
pub async fn eager_many_to_many<P, R, V, C>(parents: Vec<P>, db: &C) -> Result<Vec<(P, Vec<R>)>>
where
    P: Entity + Clone,
    R: Entity,
    V: Entity,
    C: ConnectionTrait,
    P::Sea: Related<R::Sea>,
{
    let models: Vec<<P::Sea as EntityTrait>::Model> =
        parents.iter().map(|p| p.clone().into()).collect();
    let related = models
        .load_many_to_many(R::Sea::default(), V::Sea::default(), db)
        .await
        .map_err(map_db_err)?;
    Ok(parents
        .into_iter()
        .zip(related)
        .map(|(parent, kids)| (parent, kids.into_iter().map(Into::into).collect()))
        .collect())
}
