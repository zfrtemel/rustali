//! Code-first schema model and diff engine.
//!
//! The owned [`Schema`] (a set of [`Table`]s) is the normalised form of both sides of
//! a diff: the **desired** schema, built from the application's entities
//! ([`Schema::from_tables`]), and the **current** schema, loaded from the snapshot file
//! (the `rustali-migrate` crate's territory) or pulled from the database. [`Schema::diff`]
//! computes the [`SchemaChange`]s between them; the migrate crate renders those as a
//! migration.
//!
//! This module is pure and I/O-free so it can be exhaustively unit-tested.

mod column;
mod diff;
mod types;

pub use column::{Column, Table};
pub use diff::{ColumnChange, SchemaChange, TableDiff, diff_tables};
pub use types::ColumnType;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::entity::TableMeta;

/// A whole database schema: tables keyed by name (sorted for deterministic output).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schema {
    /// Tables by name.
    pub tables: BTreeMap<String, Table>,
}

impl Schema {
    /// An empty schema (what a brand-new project diffs against).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the desired schema from entity [`TableMeta`]s.
    #[must_use]
    pub fn from_tables(tables: impl IntoIterator<Item = TableMeta>) -> Self {
        let mut map = BTreeMap::new();
        for meta in tables {
            let table = Table::from_meta(meta);
            map.insert(table.name.clone(), table);
        }
        Self { tables: map }
    }

    /// Changes that turn `self` (current) into `desired`.
    #[must_use]
    pub fn diff(&self, desired: &Self) -> Vec<SchemaChange> {
        diff::diff_schema(self, desired)
    }

    /// `true` when nothing differs.
    #[must_use]
    pub fn matches(&self, desired: &Self) -> bool {
        self.diff(desired).is_empty()
    }
}
