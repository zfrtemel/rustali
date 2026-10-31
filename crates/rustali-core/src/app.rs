//! The application builder, shared context and server bootstrap.

use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use axum::{Router, extract::Request, response::IntoResponse, routing::Route};
use tokio::net::TcpListener;
use tower::{Layer, Service};

use crate::{Config, Result, middleware, telemetry};

/// Immutable, cheaply clonable state shared by every request.
///
/// Handlers receive it through the [`crate::Ctx`] extractor or `State<AppContext>`.
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

    /// Add a tower layer around all routes registered so far.
    ///
    /// User layers sit *inside* the default stack (request id, tracing, panic
    /// catching, error negotiation), so they see the request id and their errors
    /// are still rendered consistently.
    #[must_use]
    pub fn layer<L>(mut self, layer: L) -> Self
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<Request> + Clone + Send + Sync + 'static,
        <L::Service as Service<Request>>::Response: IntoResponse + 'static,
        <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
    {
        self.router = self.router.layer(layer);
        self
    }

    /// Build the context and the fully layered router without binding a socket.
    ///
    /// Tests drive the returned [`Router`] directly with `tower::ServiceExt::oneshot`.
    pub fn build(self) -> Result<(AppContext, Router)> {
        let config = match self.config {
            Some(cfg) => cfg,
            None => Config::load()?,
        };
        let ctx = AppContext {
            inner: Arc::new(Inner { config }),
        };
        let router = middleware::apply_defaults(self.router, ctx.clone()).with_state(ctx.clone());
        Ok((ctx, router))
    }

    /// Initialise telemetry, bind the configured address and serve until
    /// Ctrl-C / SIGTERM.
    pub async fn serve(self) -> Result<()> {
        let (ctx, router) = self.build()?;
        telemetry::init(&ctx.config().log);

        let addr = SocketAddr::new(ctx.config().server.host, ctx.config().server.port);
        let listener = TcpListener::bind(addr).await?;
        tracing::info!(%addr, name = %ctx.config().app.name, env = %ctx.config().app.env, "listening");
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
