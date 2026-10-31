//! Mapping SeaORM errors onto the framework [`Error`].

use rustali_core::{Error, Result};
use sea_orm::DbErr;

/// Convert a SeaORM error into the framework error.
///
/// `RecordNotFound` becomes [`Error::NotFound`] so handlers that use
/// `exec_with_returning`-style APIs render a 404; everything else is a 500 that keeps
/// the driver error as its source.
#[must_use]
pub fn map_db_err(err: DbErr) -> Error {
    match err {
        DbErr::RecordNotFound(_) => Error::NotFound,
        other => Error::database(other),
    }
}

/// `.or_db()?` for raw SeaORM calls inside handlers.
pub trait DbResultExt<T> {
    /// Map `DbErr` to the framework error.
    fn or_db(self) -> Result<T>;
}

impl<T> DbResultExt<T> for std::result::Result<T, DbErr> {
    fn or_db(self) -> Result<T> {
        self.map_err(map_db_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_maps_to_404() {
        assert!(matches!(
            map_db_err(DbErr::RecordNotFound("x".into())),
            Error::NotFound
        ));
    }

    #[test]
    fn other_errors_are_database_errors() {
        let err = map_db_err(DbErr::Custom("boom".into()));
        assert!(matches!(err, Error::Database(_)));
        assert!(err.to_string().contains("boom"));
    }
}
