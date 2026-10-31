//! The schema snapshot: the last-applied schema, stored as JSON next to migrations.
//!
//! `migrate:diff` compares the entities' desired schema against this file to decide
//! what changed, then rewrites it. It is committed to version control so diffs are
//! reproducible and reviewable.

use std::path::{Path, PathBuf};

use rustali_core::{Error, Result};
use rustali_orm::Schema;

/// Default snapshot path, relative to the project root.
pub const DEFAULT_PATH: &str = "db/schema.snapshot.json";

/// Load the snapshot, or an empty schema when the file does not exist yet.
pub fn load(path: &Path) -> Result<Schema> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|err| Error::bad_request(format!("{}: {err}", path.display()))),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Schema::new()),
        Err(err) => Err(Error::internal(err)),
    }
}

/// Write the snapshot, pretty-printed and newline-terminated, creating parent dirs.
pub fn save(path: &Path, schema: &Schema) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::internal)?;
    }
    let mut json = serde_json::to_string_pretty(schema).map_err(Error::internal)?;
    json.push('\n');
    std::fs::write(path, json).map_err(Error::internal)
}

/// The migrations directory that holds the snapshot.
#[must_use]
pub fn migrations_dir(snapshot: &Path) -> PathBuf {
    snapshot
        .parent()
        .map_or_else(|| PathBuf::from("db"), |p| p.join("migrations"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use rustali_orm::{Schema, TableMeta, entity::ColumnMeta};

    use super::*;

    fn sample() -> Schema {
        static COLS: &[ColumnMeta] = &[ColumnMeta {
            name: "id",
            rust_type: "i64",
            nullable: false,
            primary_key: true,
            auto_increment: true,
            unique: false,
            max_len: None,
        }];
        Schema::from_tables([TableMeta {
            table: "users",
            columns: COLS,
        }])
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = std::env::temp_dir().join(format!("rustali-snap-{}", std::process::id()));
        let path = dir.join("schema.snapshot.json");
        let schema = sample();
        save(&path, &schema).unwrap();
        assert_eq!(load(&path).unwrap(), schema);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_file_is_an_empty_schema() {
        let path = Path::new("definitely/does/not/exist/schema.snapshot.json");
        assert_eq!(load(path).unwrap(), Schema::new());
    }
}
