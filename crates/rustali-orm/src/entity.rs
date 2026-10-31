//! The [`Entity`] trait implemented by `#[derive(Entity)]`.
//!
//! A Rustali entity is a plain struct — the thing you pass around, serialise and
//! render. The derive generates a hidden SeaORM entity module next to it and
//! implements [`Entity`] to bridge the two; [`crate::Query`] uses that bridge to
//! return your structs from any query.

use sea_orm::{EntityTrait, PrimaryKeyTrait};

/// Static description of one column, used by schema tooling and code-first diffs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // a descriptor of independent flags, not a state machine
pub struct ColumnMeta {
    /// Column name in the database.
    pub name: &'static str,
    /// Rust type as written in the struct, e.g. `Option<String>`.
    pub rust_type: &'static str,
    /// `true` for `Option<T>` fields.
    pub nullable: bool,
    /// Part of the primary key.
    pub primary_key: bool,
    /// Database-generated value (auto-increment); omitted on insert.
    pub auto_increment: bool,
    /// Unique constraint requested on the column.
    pub unique: bool,
    /// Maximum length for string columns (`#[column(max_len = N)]`).
    pub max_len: Option<u32>,
}

/// Static schema of one entity's table, the input to the code-first diff engine
/// ([`crate::schema`]). Produced by `#[derive(Entity)]` via [`Entity::schema`].
#[derive(Debug, Clone, Copy)]
pub struct TableMeta {
    /// Table name.
    pub table: &'static str,
    /// Columns in declaration order.
    pub columns: &'static [ColumnMeta],
}

/// Bridge between a plain struct and its generated SeaORM entity.
pub trait Entity:
    Sized
    + Send
    + Sync
    + 'static
    + From<<Self::Sea as EntityTrait>::Model>
    + Into<<Self::Sea as EntityTrait>::Model>
{
    /// The generated SeaORM entity (`your_struct_entity::Entity`).
    type Sea: EntityTrait<Model: Send + Sync>;

    /// Primary-key type as declared on the struct.
    type Key: Into<<<Self::Sea as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType>
        + Clone
        + Send
        + Sync;

    /// Table name.
    const TABLE: &'static str;

    /// Column metadata in declaration order.
    const COLUMNS: &'static [ColumnMeta];

    /// This entity's table as a [`TableMeta`], for the code-first diff engine.
    #[must_use]
    fn schema() -> TableMeta {
        TableMeta {
            table: Self::TABLE,
            columns: Self::COLUMNS,
        }
    }

    /// Primary key of this row.
    fn key(&self) -> Self::Key;

    /// Active model for an `INSERT`: auto-increment keys are `NotSet`, everything else
    /// `Set`.
    fn into_active_insert(self) -> <Self::Sea as EntityTrait>::ActiveModel;

    /// Active model for an `UPDATE`: primary key `Unchanged`, everything else `Set`.
    fn into_active_update(self) -> <Self::Sea as EntityTrait>::ActiveModel;
}
