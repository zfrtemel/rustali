//! Explicit dependency injection: the service registry and the [`Provider`] trait.
//!
//! Services are registered by **type** at startup, looked up by type at runtime, with no
//! string binding and no auto-wiring magic — the type system is the container. Packages ship
//! a [`Provider`] that reads configuration and registers what it owns; handlers receive
//! services through the [`Inject<T>`](crate::extract::Inject) extractor.

use std::{
    any::{Any, TypeId, type_name},
    collections::HashMap,
    fmt,
    future::Future,
    pin::Pin,
    sync::Arc,
};

use crate::{Config, Error, Result, command::Commands};

/// Type-indexed storage for shared, immutable services.
#[derive(Default, Clone)]
pub struct Services {
    map: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    names: Vec<&'static str>,
}

impl Services {
    /// Register `value`, replacing any previous service of the same type.
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) {
        self.map.insert(TypeId::of::<T>(), Arc::new(value));
        self.names.push(type_name::<T>());
    }

    /// Look up a service by type.
    #[must_use]
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|any| Arc::clone(any).downcast::<T>().ok())
    }

    /// Like [`Self::get`] but fails with a descriptive error.
    pub fn require<T: Send + Sync + 'static>(&self) -> Result<Arc<T>> {
        self.get::<T>()
            .ok_or_else(|| Error::internal(MissingService(type_name::<T>())))
    }

    /// `true` when a service of type `T` is registered.
    #[must_use]
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }

    /// Type names of every registered service, in registration order.
    #[must_use]
    pub fn names(&self) -> &[&'static str] {
        &self.names
    }
}

impl fmt::Debug for Services {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.names).finish()
    }
}

/// Error raised when a handler asks for a service nobody registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("service `{0}` is not registered; add its provider to App::builder()")]
pub struct MissingService(pub &'static str);

/// Everything a [`Provider`] may use while registering: the configuration and the
/// registry being built.
pub struct Registry<'a> {
    config: &'a Config,
    services: &'a mut Services,
    commands: &'a mut Commands,
}

impl<'a> Registry<'a> {
    pub(crate) const fn new(
        config: &'a Config,
        services: &'a mut Services,
        commands: &'a mut Commands,
    ) -> Self {
        Self {
            config,
            services,
            commands,
        }
    }

    /// The loaded configuration.
    #[must_use]
    pub const fn config(&self) -> &Config {
        self.config
    }

    /// Register a service.
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) {
        self.services.insert(value);
    }

    /// Register a console command (see [`crate::command::Command`]).
    pub fn command(&mut self, command: impl crate::command::Command) {
        self.commands.insert(command);
    }

    /// Fetch a service registered by an earlier provider.
    pub fn require<T: Send + Sync + 'static>(&self) -> Result<Arc<T>> {
        self.services.require::<T>()
    }
}

/// Future returned by [`Provider::register`].
pub type RegisterFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

/// A package's startup hook, run once by `App::builder().provider(..)` in order.
///
/// Providers are async because registering often means connecting (database pool,
/// cache). Implementations should read their own configuration section from
/// [`Registry::config`] and fail loudly with [`Error::Config`] when it is invalid.
///
/// ```ignore
/// struct ClockProvider;
///
/// impl Provider for ClockProvider {
///     fn register<'a>(&'a self, registry: &'a mut Registry<'_>) -> RegisterFuture<'a> {
///         Box::pin(async move {
///             registry.insert(SystemClock::default());
///             Ok(())
///         })
///     }
/// }
/// ```
pub trait Provider: Send + Sync + 'static {
    /// Human-readable name for logs and `rustali doctor`.
    fn name(&self) -> &'static str {
        type_name::<Self>()
    }

    /// Register services.
    fn register<'a>(&'a self, registry: &'a mut Registry<'_>) -> RegisterFuture<'a>;
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Clock(u64);

    #[test]
    fn insert_get_require() {
        let mut services = Services::default();
        assert!(services.get::<Clock>().is_none());
        services.insert(Clock(7));
        assert_eq!(*services.get::<Clock>().unwrap(), Clock(7));
        assert!(services.contains::<Clock>());
        assert!(services.require::<String>().is_err());
    }

    #[test]
    fn missing_service_error_names_the_type() {
        let services = Services::default();
        let err = services.require::<Clock>().unwrap_err();
        assert!(err.to_string().contains("Clock"), "{err}");
    }
}
