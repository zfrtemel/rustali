//! The diff algorithm: current schema → desired schema ⇒ ordered changes.

use serde::{Deserialize, Serialize};

use super::{Column, Schema, Table};

/// One top-level schema change. Ordered so that a generated migration applies safely
/// (creates before alters, drops tables last).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum SchemaChange {
    /// A new table with all its columns.
    CreateTable(Table),
    /// Column-level changes to an existing table.
    AlterTable(TableDiff),
    /// A table present in the current schema but not desired. Destructive.
    DropTable {
        /// Name of the table to drop.
        name: String,
    },
}

impl SchemaChange {
    /// `true` if applying this change can lose data.
    #[must_use]
    pub fn is_destructive(&self) -> bool {
        match self {
            Self::CreateTable(_) => false,
            Self::DropTable { .. } => true,
            Self::AlterTable(diff) => diff.changes.iter().any(ColumnChange::is_destructive),
        }
    }

    /// Short human-readable summary for migration file names and logs.
    #[must_use]
    pub fn summary(&self) -> String {
        match self {
            Self::CreateTable(t) => format!("create_{}", t.name),
            Self::DropTable { name } => format!("drop_{name}"),
            Self::AlterTable(d) => format!("alter_{}", d.table),
        }
    }
}

/// Column-level changes within one table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableDiff {
    /// Table being altered.
    pub table: String,
    /// The changes, add/modify before drop.
    pub changes: Vec<ColumnChange>,
}

/// One column-level change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ColumnChange {
    /// Add a column. If non-nullable without a default, the generator warns.
    AddColumn(Column),
    /// Replace a column definition (type / nullability / uniqueness changed).
    AlterColumn {
        /// Previous definition.
        from: Column,
        /// New definition.
        to: Column,
    },
    /// Drop a column. Destructive.
    DropColumn {
        /// Name of the column to drop.
        name: String,
    },
}

impl ColumnChange {
    /// `true` if applying this can lose data.
    #[must_use]
    pub fn is_destructive(&self) -> bool {
        match self {
            Self::AddColumn(col) => !col.nullable && !col.auto_increment,
            Self::DropColumn { .. } => true,
            // A type change can truncate/fail; tightening NULL → NOT NULL can reject
            // existing rows. Widening to nullable is safe.
            Self::AlterColumn { from, to } => from.ty != to.ty || (from.nullable && !to.nullable),
        }
    }
}

/// Diff two whole schemas. Order: creates, alters, then drops.
#[must_use]
pub(crate) fn diff_schema(current: &Schema, desired: &Schema) -> Vec<SchemaChange> {
    let mut creates = Vec::new();
    let mut alters = Vec::new();
    let mut drops = Vec::new();

    for (name, desired_table) in &desired.tables {
        match current.tables.get(name) {
            None => creates.push(SchemaChange::CreateTable(desired_table.clone())),
            Some(current_table) => {
                if let Some(diff) = diff_tables(current_table, desired_table) {
                    alters.push(SchemaChange::AlterTable(diff));
                }
            }
        }
    }
    for name in current.tables.keys() {
        if !desired.tables.contains_key(name) {
            drops.push(SchemaChange::DropTable { name: name.clone() });
        }
    }

    creates.into_iter().chain(alters).chain(drops).collect()
}

/// Diff one table. `None` when identical.
#[must_use]
pub fn diff_tables(current: &Table, desired: &Table) -> Option<TableDiff> {
    let mut adds = Vec::new();
    let mut alters = Vec::new();
    let mut drops = Vec::new();

    for col in &desired.columns {
        match current.column(&col.name) {
            None => adds.push(ColumnChange::AddColumn(col.clone())),
            Some(existing) if existing != col => {
                alters.push(ColumnChange::AlterColumn {
                    from: existing.clone(),
                    to: col.clone(),
                });
            }
            Some(_) => {}
        }
    }
    for col in &current.columns {
        if desired.column(&col.name).is_none() {
            drops.push(ColumnChange::DropColumn {
                name: col.name.clone(),
            });
        }
    }

    let changes: Vec<ColumnChange> = adds.into_iter().chain(alters).chain(drops).collect();
    if changes.is_empty() {
        None
    } else {
        Some(TableDiff {
            table: desired.name.clone(),
            changes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::ColumnType;

    fn col(name: &str, ty: ColumnType, nullable: bool) -> Column {
        Column {
            name: name.into(),
            ty,
            nullable,
            primary_key: false,
            auto_increment: false,
            unique: false,
        }
    }

    fn pk() -> Column {
        Column {
            name: "id".into(),
            ty: ColumnType::BigInt,
            nullable: false,
            primary_key: true,
            auto_increment: true,
            unique: false,
        }
    }

    fn table(name: &str, columns: Vec<Column>) -> Table {
        Table {
            name: name.into(),
            columns,
        }
    }

    fn schema(tables: Vec<Table>) -> Schema {
        Schema {
            tables: tables.into_iter().map(|t| (t.name.clone(), t)).collect(),
        }
    }

    #[test]
    fn new_table_is_a_create() {
        let desired = schema(vec![table("users", vec![pk()])]);
        let changes = diff_schema(&Schema::new(), &desired);
        assert_eq!(changes.len(), 1);
        assert!(matches!(&changes[0], SchemaChange::CreateTable(t) if t.name == "users"));
        assert!(!changes[0].is_destructive());
    }

    #[test]
    fn removed_table_is_a_destructive_drop() {
        let current = schema(vec![table("users", vec![pk()])]);
        let changes = diff_schema(&current, &Schema::new());
        assert_eq!(
            changes,
            vec![SchemaChange::DropTable {
                name: "users".into()
            }]
        );
        assert!(changes[0].is_destructive());
    }

    #[test]
    fn added_column_is_detected() {
        let current = schema(vec![table("users", vec![pk()])]);
        let desired = schema(vec![table(
            "users",
            vec![pk(), col("email", ColumnType::Text { max_len: None }, true)],
        )]);
        let changes = diff_schema(&current, &desired);
        let SchemaChange::AlterTable(diff) = &changes[0] else {
            panic!("expected alter")
        };
        assert_eq!(
            diff.changes,
            vec![ColumnChange::AddColumn(col(
                "email",
                ColumnType::Text { max_len: None },
                true
            ))]
        );
        assert!(
            !changes[0].is_destructive(),
            "adding a nullable column is safe"
        );
    }

    #[test]
    fn adding_a_required_column_is_destructive() {
        let current = schema(vec![table("users", vec![pk()])]);
        let desired = schema(vec![table(
            "users",
            vec![pk(), col("age", ColumnType::Int, false)],
        )]);
        assert!(diff_schema(&current, &desired)[0].is_destructive());
    }

    #[test]
    fn type_change_is_a_destructive_alter() {
        let current = schema(vec![table(
            "users",
            vec![pk(), col("age", ColumnType::SmallInt, false)],
        )]);
        let desired = schema(vec![table(
            "users",
            vec![pk(), col("age", ColumnType::BigInt, false)],
        )]);
        let changes = diff_schema(&current, &desired);
        let SchemaChange::AlterTable(diff) = &changes[0] else {
            panic!()
        };
        assert!(matches!(diff.changes[0], ColumnChange::AlterColumn { .. }));
        assert!(changes[0].is_destructive());
    }

    #[test]
    fn widening_nullability_is_non_destructive() {
        let current = schema(vec![table(
            "t",
            vec![col("note", ColumnType::Text { max_len: None }, false)],
        )]);
        let desired = schema(vec![table(
            "t",
            vec![col("note", ColumnType::Text { max_len: None }, true)],
        )]);
        assert!(!diff_schema(&current, &desired)[0].is_destructive());
    }

    #[test]
    fn dropped_column_is_destructive() {
        let current = schema(vec![table(
            "users",
            vec![pk(), col("tmp", ColumnType::Int, true)],
        )]);
        let desired = schema(vec![table("users", vec![pk()])]);
        let changes = diff_schema(&current, &desired);
        let SchemaChange::AlterTable(diff) = &changes[0] else {
            panic!()
        };
        assert_eq!(
            diff.changes,
            vec![ColumnChange::DropColumn { name: "tmp".into() }]
        );
        assert!(changes[0].is_destructive());
    }

    #[test]
    fn identical_schemas_produce_no_changes() {
        let s = schema(vec![table(
            "users",
            vec![
                pk(),
                col("email", ColumnType::Text { max_len: Some(200) }, false),
            ],
        )]);
        assert_eq!(diff_schema(&s, &s), vec![]);
        assert!(s.matches(&s));
    }

    #[test]
    fn changes_are_ordered_create_alter_drop() {
        let current = schema(vec![table("keep", vec![pk()]), table("gone", vec![pk()])]);
        let desired = schema(vec![
            table("keep", vec![pk(), col("x", ColumnType::Int, true)]),
            table("fresh", vec![pk()]),
        ]);
        let kinds: Vec<&str> = diff_schema(&current, &desired)
            .iter()
            .map(|c| match c {
                SchemaChange::CreateTable(_) => "create",
                SchemaChange::AlterTable(_) => "alter",
                SchemaChange::DropTable { .. } => "drop",
            })
            .collect();
        assert_eq!(kinds, ["create", "alter", "drop"]);
    }
}
