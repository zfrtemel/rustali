//! Testing helpers for the Rustali web framework.
//!
//! Drive a built application in-process (no socket, no port) and make fluent
//! assertions on the response:
//!
//! ```ignore
//! let (_ctx, router) = App::builder().config(test_config()).controllers().build().await?;
//! let client = TestClient::new(router);
//!
//! client.get("/api/todos").await
//!     .assert_ok()
//!     .assert_json_contains("items");
//!
//! client.post_form("/posts", &[("title", "Hi"), ("body", "")]).await
//!     .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
//! ```
//!
//! Each call runs the full middleware stack through `tower::ServiceExt::oneshot`, so
//! error negotiation, sessions and CSRF behave exactly as in production. A
//! [`TestClient`] keeps cookies between calls, so a login in one request is seen by the
//! next.

mod client;
mod response;

pub use client::TestClient;
pub use response::TestResponse;

/// A `Config` tuned for tests: `test` env, debug on, an in-memory SQLite URL and a
/// fixed app key. Apps can start from this and override sections.
#[must_use]
pub fn test_config() -> rustali_core::Config {
    let mut config = rustali_core::Config::default();
    config.app.env = "test".into();
    config.app.debug = true;
    config.app.key = "test-key-at-least-sixteen-characters-long".into();
    config
}
