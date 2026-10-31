//! The [`UserStore`] abstraction and the auth provider.

use std::{future::Future, pin::Pin, sync::Arc};

use axum::http::request::Parts;
use rustali_core::{
    AppContext, Provider, RegisterFuture, Registry, Result,
    auth::{ActorRef, ActorResolver, ActorResolverHandle},
};

use crate::{Authenticatable, guard::Guard};

/// Future returned by [`UserStore`] lookups.
pub type StoreFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// Loads users for the guards. An application implements this (usually one line over
/// its `#[derive(Entity)]` user) and registers it with [`AuthProvider`].
pub trait UserStore: Send + Sync + 'static {
    /// The application's user type.
    type User: Authenticatable;

    /// Find a user by the identifier stored in a session/token (`auth_id`).
    fn find_by_id<'a>(
        &'a self,
        ctx: &'a AppContext,
        id: &'a str,
    ) -> StoreFuture<'a, Option<Self::User>>;

    /// Find a user by the login field (usually email), for credential checks.
    fn find_by_login<'a>(
        &'a self,
        ctx: &'a AppContext,
        login: &'a str,
    ) -> StoreFuture<'a, Option<Self::User>>;
}

/// Type-erased [`UserStore`] stored as a service so guards can share it.
pub(crate) trait DynUserStore: Send + Sync + 'static {
    fn find_by_id_dyn<'a>(
        &'a self,
        ctx: &'a AppContext,
        id: &'a str,
    ) -> StoreFuture<'a, Option<ActorRef>>;
}

struct ErasedStore<S>(Arc<S>);

impl<S: UserStore> DynUserStore for ErasedStore<S> {
    fn find_by_id_dyn<'a>(
        &'a self,
        ctx: &'a AppContext,
        id: &'a str,
    ) -> StoreFuture<'a, Option<ActorRef>> {
        Box::pin(async move {
            let user = self.0.find_by_id(ctx, id).await?;
            Ok(user.map(crate::user::AuthUser::actor))
        })
    }
}

/// Service wrapper around the erased store.
pub(crate) struct UserStoreHandle(pub(crate) Arc<dyn DynUserStore>);

/// Registers authentication: the user store, the configured guards and an
/// [`ActorResolver`] so `#[authorize]` sees the current user.
///
/// ```ignore
/// App::builder()
///     .provider(DatabaseProvider)
///     .provider(SessionProvider)
///     .provider(AuthProvider::new(Users))   // Users: UserStore
///     .configure(session_layers)
///     .controllers()
///     .run()
///     .await
/// ```
pub struct AuthProvider<S: UserStore> {
    store: Arc<S>,
    guards: Vec<Guard>,
}

impl<S: UserStore> AuthProvider<S> {
    /// Use `store`, with the default guard chain: session first, then bearer token.
    pub fn new(store: S) -> Self {
        Self {
            store: Arc::new(store),
            guards: vec![Guard::Session, Guard::Bearer],
        }
    }

    /// Replace the guard chain (tried in order; first match wins).
    #[must_use]
    pub fn guards(mut self, guards: impl IntoIterator<Item = Guard>) -> Self {
        self.guards = guards.into_iter().collect();
        self
    }
}

impl<S: UserStore> Provider for AuthProvider<S> {
    fn name(&self) -> &'static str {
        "auth"
    }

    fn register<'a>(&'a self, registry: &'a mut Registry<'_>) -> RegisterFuture<'a> {
        Box::pin(async move {
            let handle = UserStoreHandle(Arc::new(ErasedStore(Arc::clone(&self.store))));
            registry.insert(handle);
            registry.insert(GuardChain(self.guards.clone()));
            registry.insert(ActorResolverHandle(Box::new(ChainResolver)));
            Ok(())
        })
    }
}

/// The configured guard order, stored as a service.
#[derive(Clone)]
pub(crate) struct GuardChain(pub(crate) Vec<Guard>);

/// Resolves the actor by trying each configured guard in turn.
struct ChainResolver;

impl ActorResolver for ChainResolver {
    fn resolve<'a>(
        &'a self,
        ctx: &'a AppContext,
        parts: &'a Parts,
    ) -> rustali_core::auth::AuthFuture<'a, Option<ActorRef>> {
        Box::pin(async move {
            let Ok(chain) = ctx.service::<GuardChain>() else {
                return Ok(None);
            };
            let Ok(store) = ctx.service::<UserStoreHandle>() else {
                return Ok(None);
            };
            for guard in &chain.0 {
                if let Some(id) = guard.extract_id(ctx, parts).await?
                    && let Some(actor) = store.0.find_by_id_dyn(ctx, &id).await?
                {
                    tracing::debug!(guard = guard.name(), %id, "authenticated");
                    return Ok(Some(actor));
                }
            }
            Ok(None)
        })
    }
}
