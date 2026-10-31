//! Create the `todos` table.

// `#[async_trait]` requires the impl signature to match the trait exactly, and
// SeaORM declares `manager: &SchemaManager` with an elided lifetime.
#![allow(elided_lifetimes_in_paths)]

use rustali::migrate::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Todos::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Todos::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Todos::Title).string_len(200).not_null())
                    .col(
                        ColumnDef::new(Todos::Done)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Todos::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Todos::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Todos::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Todos {
    Table,
    Id,
    Title,
    Done,
    CreatedAt,
    UpdatedAt,
}
