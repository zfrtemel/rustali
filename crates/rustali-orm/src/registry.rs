//! Compile-time registry of entities, for the code-first diff.
//!
//! `#[derive(Entity)]` submits an [`EntityRegistration`] to an [`inventory`]
//! collection, so `migrate:diff` can build the desired [`Schema`] from every entity
//! linked into the binary — no manual list to keep in sync.

use crate::{TableMeta, schema::Schema};

/// One entity's contribution to the schema, registered by the derive.
pub struct EntityRegistration {
    /// Returns the entity's table metadata.
    pub schema: fn() -> TableMeta,
}

inventory::collect!(EntityRegistration);

/// Table metadata for every registered entity, in link order.
pub fn registered_tables() -> impl Iterator<Item = TableMeta> {
    inventory::iter::<EntityRegistration>
        .into_iter()
        .map(|reg| (reg.schema)())
}

/// The desired schema, assembled from every registered entity.
#[must_use]
pub fn desired_schema() -> Schema {
    Schema::from_tables(registered_tables())
}
