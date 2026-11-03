//! Migrations.

use rustali::migrate::prelude::*;

mod m20261101_000001_create_users;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20261101_000001_create_users::Migration)]
    }
}
