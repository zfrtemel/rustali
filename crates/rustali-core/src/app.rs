//! The application builder, shared context, server bootstrap and command runner.

use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use axum::{Router, extract::Request, response::IntoResponse, routing::Route};
use tokio::net::TcpListener;
use tower::{Layer, Service};

use crate::{
    Config, Result,
    command::{Command, Commands, UnknownCommand},
    middleware,
    services::{Provider, Registry, Services},
    telemetry,
};

/// Immutable, cheaply clonable state shared by every request.
///
/// Handlers receive it through the [`crate::Ctx`] extractor or `State<AppContext>`;
/// individual services through [`crate::Inject`].
#[derive(Clone)]
pub struct AppContext {
    inner: Arc<Inner>,
}

struct Inner {
    config: Config,
    services: Services,
    commands: Commands,
}

impl AppContext {
    /// The loaded configuration.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    /// Registered services.
    #[must_use]
    pub fn services(&self) -> &Services {
        &self.inner.services
    }

    /// Shortcut for `services().require::<T>()`.
    pub fn service<T: Send + Sync + 'static>(&self) -> Result<Arc<T>> {
        self.inner.services.require::<T>()
    }

    /// Registered console commands.
    #[must_use]
    pub fn commands(&self) -> &Commands {
        &self.inner.commands
    }
}

impl std::fmt::Debug for AppContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppContext")
            .field("config", &self.inner.config)
            .field("services", &self.inner.services)
            .field("commands", &self.inner.commands)
            .finish()
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

/// Fluent builder that assembles configuration, providers, routes, middleware and
/// commands, then serves or runs a command.
#[derive(Default)]
pub struct AppBuilder {
    config: Option<Config>,
    providers: Vec<Box<dyn Provider>>,
    services: Services,
    commands: Commands,
    router: Router<AppContext>,
}

impl AppBuilder {
    /// Use an already loaded configuration instead of [`Config::load`].
    #[must_use]
    pub fn config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    /// Add a provider. Providers run in the order they were added, after the
    /// configuration is loaded and before routes are built.
    #[must_use]
    pub fn provider(mut self, provider: impl Provider) -> Self {
        self.providers.push(Box::new(provider));
        self
    }

    /// Register a ready-made service without a provider.
    #[must_use]
    pub fn provide<T: Send + Sync + 'static>(mut self, value: T) -> Self {
        self.services.insert(value);
        self
    }

    /// Register a console command.
    #[must_use]
    pub fn command(mut self, command: impl Command) -> Self {
        self.commands.insert(command);
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

    /// Load configuration, run providers and assemble the fully layered router
    /// without binding a socket.
    ///
    /// Tests drive the returned [`Router`] directly with `tower::ServiceExt::oneshot`.
    pub async fn build(self) -> Result<(AppContext, Router)> {
        let config = match self.config {
            Some(cfg) => cfg,
            None => Config::load()?,
        };

        let mut services = self.services;
        let mut commands = self.commands;
        for provider in &self.providers {
            tracing::debug!(provider = provider.name(), "registering");
            let mut registry = Registry::new(&config, &mut services, &mut commands);
            provider.register(&mut registry).await?;
        }

        let ctx = AppContext {
            inner: Arc::new(Inner {
                config,
                services,
                commands,
            }),
        };
        let router = middleware::apply_defaults(self.router, ctx.clone()).with_state(ctx.clone());
        Ok((ctx, router))
    }

    /// Initialise telemetry, run providers, bind the configured address and serve
    /// until Ctrl-C / SIGTERM.
    pub async fn serve(self) -> Result<()> {
        let log_config = self.log_config()?;
        telemetry::init(&log_config);

        let (ctx, router) = self.build().await?;
        let addr = SocketAddr::new(ctx.config().server.host, ctx.config().server.port);
        let listener = TcpListener::bind(addr).await?;
        tracing::info!(%addr, name = %ctx.config().app.name, env = %ctx.config().app.env, "listening");
        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
        Ok(())
    }

    /// Behave as a CLI: with no arguments (or `serve`) start the server; otherwise
    /// run the named command with the remaining arguments. This is what a
    /// generated `main.rs` calls.
    pub async fn run(self) -> Result<()> {
        let args: Vec<String> = std::env::args().collect();
        self.run_with(args).await
    }

    /// Like [`Self::run`] with explicit arguments (`args[0]` is the binary name).
    pub async fn run_with(self, args: Vec<String>) -> Result<()> {
        let bin = args.first().map_or("app", |s| s.as_str()).to_owned();
        let mut rest = args.into_iter().skip(1);
        let Some(name) = rest.next() else {
            return self.serve().await;
        };
        match name.as_str() {
            "serve" => self.serve().await,
            "help" | "--help" | "-h" => {
                let (ctx, _) = self.build().await?;
                println!("{}", ctx.commands().help(&bin));
                Ok(())
            }
            other => {
                let log_config = self.log_config()?;
                telemetry::init(&log_config);
                let (ctx, _) = self.build().await?;
                let command = ctx
                    .commands()
                    .get(other)
                    .ok_or_else(|| UnknownCommand(other.to_owned()))?;
                command.run(&ctx, rest.collect()).await
            }
        }
    }

    fn log_config(&self) -> Result<crate::config::LogConfig> {
        // Telemetry must come up before providers run; when the caller did not pass a
        // config this loads it once more, which is cheap.
        Ok(match &self.config {
            Some(cfg) => cfg.log.clone(),
            None => Config::load()?.log,
        })
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
