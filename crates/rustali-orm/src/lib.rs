//! Data layer for the Rustali web framework.
//!
//! The engine is [SeaORM](https://www.sea-ql.org/SeaORM/); what Rustali adds is the
//! developer experience: one plain struct per table (`#[derive(Entity)]`, .NET POCO
//! style), a [`DatabaseProvider`] that wires the pool from `config/database.toml`, a
//! [`Db`] extractor, a typed [`Query`] builder that returns your structs and a
//! serialisable [`Page`] for pagination.
//!
//! SeaORM itself stays reachable through [`sea_orm`] for anything the sugar does not
//! cover — there is no second query language to learn.

pub mod base;
pub mod config;
pub mod entity;
pub mod error;
pub mod extract;
pub mod page;
pub mod provider;
pub mod query;
pub mod registry;
pub mod relations;
pub mod schema;

pub use chrono;
pub use sea_orm;

pub use base::{BaseEntity, SoftDelete};
pub use config::DatabaseConfig;
pub use entity::{ColumnMeta, Entity, TableMeta};
pub use error::{DbResultExt, map_db_err};
pub use extract::Db;
pub use page::Page;
pub use provider::{Database, DatabaseProvider, DbExt};
pub use query::Query;
pub use registry::{EntityRegistration, desired_schema, registered_tables};
pub use schema::{Schema, SchemaChange};

/// Paths used by `#[derive(Entity)]` output. Not public API.
#[doc(hidden)]
pub mod __private {
    pub use inventory;
    pub use rustali_core::{Error, Result};

    pub use crate::{TableMeta, registry::EntityRegistration, relations};
}

/// Items generated code and application models usually import.
pub mod prelude {
    pub use sea_orm::{
        ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, NotSet,
        PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait, Unchanged,
    };

    pub use crate::{BaseEntity, Db, DbExt, DbResultExt, Entity, Page, Query, SoftDelete};
}
