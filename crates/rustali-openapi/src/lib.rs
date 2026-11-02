//! Automatic OpenAPI + offline Swagger UI for the [Rustali](https://github.com/zfrtemel/rustali)
//! web framework.
//!
//! Like .NET's zero-config Swagger: mount it and your API documents itself. There is no
//! per-route annotation — the spec is generated from the framework's compile-time route
//! registry (the same data `cargo run -- routes` prints), so every `#[controller]` route shows
//! up automatically.
//!
//! Mount the routes when you build the app:
//!
//! ```no_run
//! # use rustali_openapi::openapi_router;
//! // App::builder()
//! //     .controllers()
//! //     .routes(openapi_router())
//! //     .run().await
//! ```
//!
//! Then open `/swagger` for the UI and `/openapi.json` for the document.
//!
//! **Scope:** paths, methods, operation ids, path parameters, tags and a security note are
//! generated. Request/response body schemas are not (the registry records no type info); the
//! design leaves room to add them later. Only `#[controller]` routes appear — routers merged
//! with `.routes(...)` are not in the registry.

mod config;
mod spec;
mod ui;

use std::sync::OnceLock;

use axum::{
    Router,
    extract::Path,
    response::{Html, IntoResponse, Json, Response},
    routing::get,
};
use rustali_core::{AppContext, Ctx, router::RouteTable};

pub use config::OpenApiConfig;
pub use spec::build_spec;

/// The generated spec, built once on first request (routes and config are fixed after
/// startup).
static SPEC: OnceLock<serde_json::Value> = OnceLock::new();

/// A router serving `GET /openapi.json`, `GET /swagger` and the vendored UI assets.
///
/// Merge it into your app with `App::builder().routes(openapi_router())`.
pub fn openapi_router() -> Router<AppContext> {
    Router::new()
        .route("/openapi.json", get(spec_handler))
        .route("/swagger", get(ui_handler))
        .route("/swagger/{file}", get(asset_handler))
}

/// `GET /openapi.json` — build (and cache) the spec from the route table.
async fn spec_handler(ctx: Ctx) -> Response {
    let spec = SPEC.get_or_init(|| {
        let config = ctx
            .config()
            .section_or_default::<OpenApiConfig>(OpenApiConfig::SECTION)
            .unwrap_or_default();
        ctx.service::<RouteTable>().map_or_else(
            |_| serde_json::json!({ "openapi": "3.1.0", "paths": {} }),
            |table| build_spec(&table, &config),
        )
    });
    Json(spec.clone()).into_response()
}

/// `GET /swagger` — the Swagger UI page.
async fn ui_handler(ctx: Ctx) -> Html<String> {
    let config = ctx
        .config()
        .section_or_default::<OpenApiConfig>(OpenApiConfig::SECTION)
        .unwrap_or_default();
    ui::index_html("/swagger", "/openapi.json", &config.title)
}

/// `GET /swagger/{file}` — a vendored UI asset.
async fn asset_handler(Path(file): Path<String>) -> Response {
    ui::asset(&file)
}
