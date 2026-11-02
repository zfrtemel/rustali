//! `config/openapi.toml`.

use serde::{Deserialize, Serialize};

/// OpenAPI / Swagger settings (`config/openapi.toml`).
///
/// ```toml
/// title = "My API"
/// version = "1.0.0"
/// ```
///
/// The UI is served at `/swagger` and the document at `/openapi.json` (fixed in this
/// version).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenApiConfig {
    /// Document title shown in the spec and the UI.
    pub title: String,
    /// API version string reported in the spec's `info`.
    pub version: String,
}

impl Default for OpenApiConfig {
    fn default() -> Self {
        Self {
            title: "Rustali API".into(),
            version: "0.1.0".into(),
        }
    }
}

impl OpenApiConfig {
    /// Section name inside the configuration tree.
    pub const SECTION: &'static str = "openapi";
}
