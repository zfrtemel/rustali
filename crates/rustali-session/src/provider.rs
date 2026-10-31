//! Wiring sessions, CSRF and form feedback into the application.

use std::sync::Arc;

use axum::{extract::Request, middleware::Next, response::Response};
use rustali_core::{
    AppContext, Provider, RegisterFuture, Registry, validation::FormFeedbackHandle,
};
use serde::{Deserialize, Serialize};
use tower_sessions::{
    Expiry, MemoryStore, SessionManagerLayer,
    cookie::{SameSite, time::Duration},
};

use crate::{csrf::CsrfLayer, flash::SessionFeedback};

/// `config/session.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionConfig {
    /// Cookie name.
    pub cookie: String,
    /// Session lifetime in minutes (inactivity expiry).
    pub lifetime_minutes: i64,
    /// `SameSite` policy: `lax`, `strict` or `none`.
    pub same_site: String,
    /// Enable CSRF protection middleware.
    pub csrf: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            cookie: "rustali_session".into(),
            lifetime_minutes: 120,
            same_site: "lax".into(),
            csrf: true,
        }
    }
}

impl SessionConfig {
    /// Section name.
    pub const SECTION: &'static str = "session";

    fn same_site(&self) -> SameSite {
        match self.same_site.to_ascii_lowercase().as_str() {
            "strict" => SameSite::Strict,
            "none" => SameSite::None,
            _ => SameSite::Lax,
        }
    }
}

/// Registers the session store, CSRF and form feedback.
///
/// This version uses an in-memory store (fine for development and single-node apps);
/// the cache package will add Redis- and database-backed stores. The signing key is
/// `app.key`; in development an empty key produces a throwaway key with a warning.
#[derive(Debug, Default, Clone, Copy)]
pub struct SessionProvider;

impl Provider for SessionProvider {
    fn name(&self) -> &'static str {
        "session"
    }

    fn register<'a>(&'a self, registry: &'a mut Registry<'_>) -> RegisterFuture<'a> {
        Box::pin(async move {
            let config: SessionConfig = registry
                .config()
                .section_or_default(SessionConfig::SECTION)?;
            let secure = registry.config().is_production();
            registry.insert(config);
            registry.insert(SessionSettings { secure });
            Ok(())
        })
    }
}

/// Resolved runtime settings stored for the router hook.
struct SessionSettings {
    secure: bool,
}

/// Applies the session + CSRF layers. Pass to `App::builder().configure(session_layers)`.
pub fn layers(ctx: &AppContext, router: axum::Router<AppContext>) -> axum::Router<AppContext> {
    let config = ctx
        .service::<SessionConfig>()
        .unwrap_or_else(|_| Arc::new(SessionConfig::default()));
    let secure = ctx.service::<SessionSettings>().is_ok_and(|s| s.secure);

    let store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(store)
        .with_name(config.cookie.clone())
        .with_http_only(true)
        .with_same_site(config.same_site())
        .with_secure(secure)
        .with_expiry(Expiry::OnInactivity(Duration::minutes(
            config.lifetime_minutes,
        )));

    let mut router = router;
    if config.csrf {
        router = router.layer(CsrfLayer);
    }
    // Form feedback needs the Session in extensions, so it runs inside the session layer.
    router = router.layer(axum::middleware::from_fn(attach_feedback));
    router.layer(session_layer)
}

/// Middleware: expose the session as a [`FormFeedbackHandle`] so `Validated<Form<T>>`
/// can flash errors and old input.
async fn attach_feedback(mut request: Request, next: Next) -> Response {
    if let Some(session) = request
        .extensions()
        .get::<tower_sessions::Session>()
        .cloned()
    {
        let handle = FormFeedbackHandle(Arc::new(SessionFeedback::new(session)));
        request.extensions_mut().insert(handle);
    }
    next.run(request).await
}
