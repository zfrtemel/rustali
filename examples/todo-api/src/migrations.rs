//! Database migrations: one file per migration under `src/migrations/`, listed in
//! chronological order by the `Migrator`. (Faz 6 will generate these files from the
//! entity structs with `rustali migrate:diff`.)
//!
//! The migration name shown by `migrate:status` is derived from the *file name*, so
//! files must be called `m<yyyymmdd>_<nnnnnn>_<description>.rs`.

use rustali::migrate::prelude::*;

mod m20261101_000001_create_todos;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20261101_000001_create_todos::Migration)]
    }
}
