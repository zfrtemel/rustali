//! Owned [`Column`] and [`Table`] — the normalised schema the diff compares.

use serde::{Deserialize, Serialize};

use super::types::{ColumnType, strip_option};
use crate::entity::TableMeta;

/// One column in the normalised schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)] // independent column flags, not a state machine
pub struct Column {
    /// Column name.
    pub name: String,
    /// Abstract type.
    pub ty: ColumnType,
    /// Whether NULLs are allowed.
    pub nullable: bool,
    /// Part of the primary key.
    pub primary_key: bool,
    /// Auto-increment / identity.
    pub auto_increment: bool,
    /// Has a UNIQUE constraint.
    pub unique: bool,
}

impl Column {
    /// Build from entity column metadata.
    #[must_use]
    pub fn from_meta(meta: &crate::entity::ColumnMeta) -> Self {
        Self {
            name: meta.name.to_owned(),
            ty: ColumnType::from_rust(strip_option(meta.rust_type), meta.max_len),
            nullable: meta.nullable,
            primary_key: meta.primary_key,
            auto_increment: meta.auto_increment,
            unique: meta.unique,
        }
    }
}

/// One table in the normalised schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Table {
    /// Table name.
    pub name: String,
    /// Columns in declaration order.
    pub columns: Vec<Column>,
}

impl Table {
    /// Build from entity table metadata.
    #[must_use]
    pub fn from_meta(meta: TableMeta) -> Self {
        Self {
            name: meta.table.to_owned(),
            columns: meta.columns.iter().map(Column::from_meta).collect(),
        }
    }

    /// Find a column by name.
    #[must_use]
    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// The primary-key column, if exactly one.
    #[must_use]
    pub fn primary_key(&self) -> Option<&Column> {
        self.columns.iter().find(|c| c.primary_key)
    }
}
