//! The storage-independent column type, mapped from Rust types and to `SeaQuery`.

use serde::{Deserialize, Serialize};

/// A database column type, abstracted over backends.
///
/// `#[derive(Entity)]` records the Rust type as a string; [`ColumnType::from_rust`]
/// maps it here, and the migrate crate maps this to a concrete `SeaQuery` `ColumnType`.
/// Keeping an intermediate enum means the snapshot is backend-neutral and the diff
/// compares semantics, not spelling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnType {
    /// 16-bit integer.
    SmallInt,
    /// 32-bit integer.
    Int,
    /// 64-bit integer.
    BigInt,
    /// IEEE double.
    Double,
    /// Boolean.
    Boolean,
    /// Variable-length text with an optional max length.
    Text {
        /// Maximum length (`varchar(N)`); `None` means unbounded `text`.
        max_len: Option<u32>,
    },
    /// Timestamp with time zone.
    Timestamp,
    /// Date.
    Date,
    /// UUID.
    Uuid,
    /// JSON.
    Json,
    /// Arbitrary binary.
    Bytes,
    /// A type the mapper did not recognise; carries the original Rust type so the
    /// diff still works and the generator can warn.
    Unknown {
        /// The original Rust type name.
        rust_type: String,
    },
}

impl ColumnType {
    /// Map a Rust type (as written in the struct, whitespace-stripped) to a column
    /// type. `Option<T>` is unwrapped by the caller; this sees the inner type.
    #[must_use]
    pub fn from_rust(rust_type: &str, max_len: Option<u32>) -> Self {
        let inner = strip_option(rust_type);
        match inner {
            "i8" | "i16" | "u8" | "u16" => Self::SmallInt,
            "i32" | "u32" => Self::Int,
            "i64" | "u64" | "isize" | "usize" => Self::BigInt,
            "f32" | "f64" => Self::Double,
            "bool" => Self::Boolean,
            "String" | "str" | "&str" => Self::Text { max_len },
            _ if inner.contains("DateTime") => Self::Timestamp,
            "NaiveDate" | "Date" => Self::Date,
            "Uuid" => Self::Uuid,
            "Value" | "Json" => Self::Json,
            _ if inner.starts_with("Vec<u8") => Self::Bytes,
            other => Self::Unknown {
                rust_type: other.to_owned(),
            },
        }
    }

    /// A short label for migration comments and `schema` output.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::SmallInt => "smallint".into(),
            Self::Int => "integer".into(),
            Self::BigInt => "bigint".into(),
            Self::Double => "double".into(),
            Self::Boolean => "boolean".into(),
            Self::Text { max_len: Some(n) } => format!("varchar({n})"),
            Self::Text { max_len: None } => "text".into(),
            Self::Timestamp => "timestamptz".into(),
            Self::Date => "date".into(),
            Self::Uuid => "uuid".into(),
            Self::Json => "json".into(),
            Self::Bytes => "bytes".into(),
            Self::Unknown { rust_type } => format!("?{rust_type}"),
        }
    }
}

/// Strip one layer of `Option<...>` if present.
#[must_use]
pub(crate) fn strip_option(rust_type: &str) -> &str {
    rust_type
        .strip_prefix("Option<")
        .and_then(|rest| rest.strip_suffix('>'))
        .unwrap_or(rust_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_rust_types() {
        assert_eq!(ColumnType::from_rust("i64", None), ColumnType::BigInt);
        assert_eq!(ColumnType::from_rust("i32", None), ColumnType::Int);
        assert_eq!(ColumnType::from_rust("bool", None), ColumnType::Boolean);
        assert_eq!(
            ColumnType::from_rust("String", Some(200)),
            ColumnType::Text { max_len: Some(200) }
        );
        assert_eq!(
            ColumnType::from_rust("Option<String>", None),
            ColumnType::Text { max_len: None }
        );
        assert_eq!(
            ColumnType::from_rust("DateTime<Utc>", None),
            ColumnType::Timestamp
        );
        assert_eq!(ColumnType::from_rust("Uuid", None), ColumnType::Uuid);
        assert!(matches!(
            ColumnType::from_rust("MyEnum", None),
            ColumnType::Unknown { .. }
        ));
    }

    #[test]
    fn labels_are_human_readable() {
        assert_eq!(ColumnType::BigInt.label(), "bigint");
        assert_eq!(
            ColumnType::Text { max_len: Some(50) }.label(),
            "varchar(50)"
        );
        assert_eq!(ColumnType::Text { max_len: None }.label(), "text");
    }
}
