//! The application builder, shared context and server bootstrap.

use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use tokio::net::TcpListener;

use crate::{Config, Result};

/// Immutable, cheaply clonable state shared by every request.
///
/// Handlers receive it through `State<AppContext>` (or the `Ctx` extractor added in
/// Faz 1). Services registered by providers are stored here as well.
#[derive(Clone)]
pub struct AppContext {
    inner: Arc<Inner>,
}

struct Inner {
    config: Config,
}

impl AppContext {
    /// The loaded configuration.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.inner.config
    }
}

impl std::fmt::Debug for AppContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppContext")
            .field("config", &self.inner.config)
            .finish_non_exhaustive()
    }
}

/// Entry point: `App::builder()`.
pub struct App;

impl App {
    /// Start describing an application.
    #[must_use]
    pub fn builder() -> AppBuilder {
        AppBuilder::default()
    }
}

/// Fluent builder that assembles configuration, routes and middleware, then serves.
#[derive(Default)]
pub struct AppBuilder {
    config: Option<Config>,
    router: Router<AppContext>,
}

impl AppBuilder {
    /// Use an already loaded configuration instead of [`Config::load`].
    #[must_use]
    pub fn config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    /// Merge an axum router. Attribute-routed controllers register through this
    /// same path once `rustali-router` lands.
    #[must_use]
    pub fn routes(mut self, router: Router<AppContext>) -> Self {
        self.router = self.router.merge(router);
        self
    }

    /// Build the context and router without binding a socket. Used by tests.
    pub fn build(self) -> Result<(AppContext, Router)> {
        let config = match self.config {
            Some(cfg) => cfg,
            None => Config::load()?,
        };
        let ctx = AppContext {
            inner: Arc::new(Inner { config }),
        };
        let router = self.router.with_state(ctx.clone());
        Ok((ctx, router))
    }

    /// Bind the configured address and serve until Ctrl-C / SIGTERM.
    pub async fn serve(self) -> Result<()> {
        let (ctx, router) = self.build()?;
        let addr = SocketAddr::new(ctx.config().server.host, ctx.config().server.port);
        let listener = TcpListener::bind(addr).await?;
        tracing::info!(%addr, name = %ctx.config().app.name, "listening");
        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
        Ok(())
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!(%err, "failed to listen for ctrl-c");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(err) => tracing::error!(%err, "failed to listen for SIGTERM"),
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
