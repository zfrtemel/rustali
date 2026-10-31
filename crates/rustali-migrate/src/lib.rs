//! Database migrations for the Rustali web framework.
//!
//! Migrations are ordinary [`sea_orm_migration`] migrations compiled into the
//! application. This crate adds the glue: a type-erased [`Runner`] registered as a
//! service, the `migrate*` console commands, and the [`MigrateAppExt::migrations`]
//! builder method.
//!
//! ```ignore
//! App::builder()
//!     .provider(DatabaseProvider)
//!     .migrations::<db::migrations::Migrator>()
//!     .run()
//!     .await
//! ```
//!
//! Then: `cargo run -- migrate`, `migrate:rollback --step 1`, `migrate:fresh`,
//! `migrate:status`, `migrate:reset`.

pub mod commands;
pub mod diff;
pub mod generate;
pub mod runner;
pub mod snapshot;

pub use diff::DiffCommand;
pub use runner::{MigrateAppExt, Migrations, Runner};
pub use sea_orm_migration;

/// What a migration file imports.
///
/// Besides SeaORM's migration prelude this exports the crate names `sea_orm_migration`
/// and `sea_orm`, because `DeriveMigrationName` / `DeriveIden` expand to paths starting
/// with them and an application normally depends only on `rustali`.
pub mod prelude {
    pub use sea_orm_migration;
    pub use sea_orm_migration::{prelude::*, sea_orm};
}

#[cfg(test)]
mod generator_tests {
    use rustali_orm::{Schema, TableMeta, entity::ColumnMeta};

    use crate::generate::migration_source;

    const V1: &[ColumnMeta] = &[
        ColumnMeta {
            name: "id",
            rust_type: "i64",
            nullable: false,
            primary_key: true,
            auto_increment: true,
            unique: false,
            max_len: None,
        },
        ColumnMeta {
            name: "email",
            rust_type: "String",
            nullable: false,
            primary_key: false,
            auto_increment: false,
            unique: true,
            max_len: Some(255),
        },
    ];
    const V2: &[ColumnMeta] = &[
        ColumnMeta {
            name: "id",
            rust_type: "i64",
            nullable: false,
            primary_key: true,
            auto_increment: true,
            unique: false,
            max_len: None,
        },
        ColumnMeta {
            name: "email",
            rust_type: "String",
            nullable: false,
            primary_key: false,
            auto_increment: false,
            unique: true,
            max_len: Some(255),
        },
        ColumnMeta {
            name: "bio",
            rust_type: "Option<String>",
            nullable: true,
            primary_key: false,
            auto_increment: false,
            unique: false,
            max_len: None,
        },
    ];

    #[test]
    fn create_then_add_column_generates_the_expected_statements() {
        // From empty: create table.
        let create = Schema::new().diff(&Schema::from_tables([TableMeta {
            table: "users",
            columns: V1,
        }]));
        let src = migration_source("m_create", &create);
        assert!(src.contains(".create_table("));
        assert!(
            src.contains("ColumnDef::new(Users::Email).string_len(255).not_null().unique_key()")
        );

        // V1 -> V2: add one nullable column, not a recreate.
        let v1 = Schema::from_tables([TableMeta {
            table: "users",
            columns: V1,
        }]);
        let v2 = Schema::from_tables([TableMeta {
            table: "users",
            columns: V2,
        }]);
        let changes = v1.diff(&v2);
        assert_eq!(changes.len(), 1);
        let src = migration_source("m_add_bio", &changes);
        assert!(
            src.contains(".add_column(ColumnDef::new(Users::Bio).text().null())"),
            "{src}"
        );
        assert!(
            src.contains(".drop_column(Users::Bio)"),
            "down drops the column"
        );
        assert!(
            !src.contains(".create_table("),
            "incremental change must not recreate the table"
        );
    }
}
