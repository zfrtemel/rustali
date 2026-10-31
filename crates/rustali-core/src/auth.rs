//! Authorization primitives shared by the kernel and the auth package.
//!
//! - [`Actor`]: whoever is making the request (a user, an API client). The auth
//!   package provides concrete actors; the kernel only needs an id and roles.
//! - [`ActorResolver`]: finds the current actor from the request (session cookie,
//!   bearer token…). Registered as a service by the auth package.
//! - [`Policy`] and [`Gate`]: named abilities (`post.update`) checked asynchronously.
//! - [`AuthorizeLayer`]: the tower layer emitted by `#[authorize("...")]`.
//!
//! Built-in abilities: `role:<name>` passes when the actor has that role;
//! `authenticated` passes for any actor.

use std::{
    any::Any,
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use axum::{
    extract::Request,
    http::request::Parts,
    response::{IntoResponse, Response},
};
use tower::{Layer, Service};

use crate::{AppContext, Error, Result};

/// The authenticated party behind a request.
pub trait Actor: Send + Sync + 'static {
    /// Stable identifier (user id, client id).
    fn id(&self) -> String;

    /// Role names, used by the built-in `role:<name>` ability.
    fn roles(&self) -> Vec<String> {
        Vec::new()
    }

    /// Downcast hook for policies that know the concrete type.
    fn as_any(&self) -> &dyn Any;
}

/// Shared actor handle.
pub type ActorRef = Arc<dyn Actor>;

/// Future returned by async authorization hooks.
pub type AuthFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// Finds the current actor for a request. `Ok(None)` means anonymous.
pub trait ActorResolver: Send + Sync + 'static {
    /// Inspect the request parts (cookies, headers, extensions) and return the actor.
    fn resolve<'a>(
        &'a self,
        ctx: &'a AppContext,
        parts: &'a Parts,
    ) -> AuthFuture<'a, Option<ActorRef>>;
}

/// Service wrapper for the registered resolver.
pub struct ActorResolverHandle(pub Box<dyn ActorResolver>);

/// Everything a [`Policy`] may look at.
pub struct Check<'a> {
    /// Application context (services, config, database…).
    pub ctx: &'a AppContext,
    /// Current actor, `None` when anonymous.
    pub actor: Option<&'a dyn Actor>,
    /// Request head: path parameters are in `uri`, extensions carry whatever earlier
    /// middleware stored.
    pub parts: &'a Parts,
}

/// One named ability.
///
/// ```ignore
/// struct UpdatePost;
///
/// impl Policy for UpdatePost {
///     fn ability(&self) -> &'static str { "post.update" }
///     fn allows<'a>(&'a self, check: Check<'a>) -> AuthFuture<'a, bool> {
///         Box::pin(async move {
///             let Some(actor) = check.actor else { return Ok(false) };
///             Ok(actor.roles().iter().any(|r| r == "editor"))
///         })
///     }
/// }
/// ```
pub trait Policy: Send + Sync + 'static {
    /// Ability name, e.g. `post.update`.
    fn ability(&self) -> &'static str;

    /// Decide.
    fn allows<'a>(&'a self, check: Check<'a>) -> AuthFuture<'a, bool>;
}

/// Registry of policies; stored as a service.
#[derive(Default, Clone)]
pub struct Gate {
    policies: HashMap<&'static str, Arc<dyn Policy>>,
}

impl Gate {
    /// Register a policy, replacing one for the same ability.
    pub fn define(&mut self, policy: impl Policy) {
        self.policies.insert(policy.ability(), Arc::new(policy));
    }

    /// `true` when a policy (or built-in rule) exists for `ability`.
    #[must_use]
    pub fn has(&self, ability: &str) -> bool {
        ability == "authenticated"
            || ability.starts_with("role:")
            || self.policies.contains_key(ability)
    }

    /// Evaluate `ability`. Unknown abilities are denied and logged.
    pub async fn allows(&self, ability: &str, check: Check<'_>) -> Result<bool> {
        if ability == "authenticated" {
            return Ok(check.actor.is_some());
        }
        if let Some(role) = ability.strip_prefix("role:") {
            return Ok(check
                .actor
                .is_some_and(|actor| actor.roles().iter().any(|r| r == role)));
        }
        let Some(policy) = self.policies.get(ability) else {
            tracing::warn!(ability, "no policy registered; denying");
            return Ok(false);
        };
        policy.allows(check).await
    }

    /// Registered ability names (built-ins excluded).
    pub fn abilities(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.policies.keys().copied()
    }
}

impl std::fmt::Debug for Gate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.policies.keys()).finish()
    }
}

/// Resolve the actor for `parts` using the registered resolver (if any).
pub async fn resolve_actor(ctx: &AppContext, parts: &Parts) -> Result<Option<ActorRef>> {
    match ctx.services().get::<ActorResolverHandle>() {
        Some(resolver) => resolver.0.resolve(ctx, parts).await,
        None => Ok(None),
    }
}

/// Run the full check: resolve actor, consult the gate, map to 401/403.
pub async fn authorize(ctx: &AppContext, parts: &Parts, ability: &str) -> Result<()> {
    let actor = resolve_actor(ctx, parts).await?;
    let gate = ctx.service::<Gate>()?;
    let allowed = gate
        .allows(
            ability,
            Check {
                ctx,
                actor: actor.as_deref(),
                parts,
            },
        )
        .await?;
    match (allowed, actor) {
        (true, _) => Ok(()),
        (false, None) => Err(Error::Unauthorized),
        (false, Some(_)) => Err(Error::Forbidden),
    }
}

/// Tower layer that enforces an ability before the inner service runs.
///
/// Emitted by `#[authorize("ability")]`; usable by hand on any axum handler or
/// router (`.layer(AuthorizeLayer::new("role:admin"))`). Requires the default
/// middleware stack, which stores the [`AppContext`] in request extensions.
#[derive(Debug, Clone, Copy)]
pub struct AuthorizeLayer {
    ability: &'static str,
}

impl AuthorizeLayer {
    /// Require `ability`.
    #[must_use]
    pub const fn new(ability: &'static str) -> Self {
        Self { ability }
    }
}

impl<S> Layer<S> for AuthorizeLayer {
    type Service = AuthorizeService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthorizeService {
            inner,
            ability: self.ability,
        }
    }
}

/// Service produced by [`AuthorizeLayer`].
#[derive(Debug, Clone)]
pub struct AuthorizeService<S> {
    inner: S,
    ability: &'static str,
}

impl<S> Service<Request> for AuthorizeService<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = std::result::Result<Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        // Take the ready inner service and leave a clone behind (tower idiom).
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let ability = self.ability;

        Box::pin(async move {
            let (parts, body) = request.into_parts();
            let Some(ctx) = parts.extensions.get::<AppContext>().cloned() else {
                tracing::error!("AuthorizeLayer used without the default middleware stack");
                return Ok(Error::internal(MissingContext).into_response());
            };
            match authorize(&ctx, &parts, ability).await {
                Ok(()) => inner.call(Request::from_parts(parts, body)).await,
                Err(err) => Ok(err.into_response()),
            }
        })
    }
}

/// Raised when a layer needs the [`AppContext`] but it was not injected.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("AppContext missing from request extensions; is the default middleware stack active?")]
pub struct MissingContext;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    struct User(&'static [&'static str]);
    impl Actor for User {
        fn id(&self) -> String {
            "1".into()
        }
        fn roles(&self) -> Vec<String> {
            self.0.iter().map(|s| (*s).to_owned()).collect()
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    struct Always(bool);
    impl Policy for Always {
        fn ability(&self) -> &'static str {
            "thing.do"
        }
        fn allows<'a>(&'a self, _: Check<'a>) -> AuthFuture<'a, bool> {
            let v = self.0;
            Box::pin(async move { Ok(v) })
        }
    }

    fn check<'a>(ctx: &'a AppContext, parts: &'a Parts, actor: Option<&'a dyn Actor>) -> Check<'a> {
        Check { ctx, actor, parts }
    }

    fn parts() -> Parts {
        axum::http::Request::builder()
            .uri("/")
            .body(())
            .unwrap()
            .into_parts()
            .0
    }

    async fn ctx() -> AppContext {
        crate::App::builder()
            .config(crate::Config::default())
            .build()
            .await
            .unwrap()
            .0
    }

    #[tokio::test]
    async fn built_in_abilities() {
        let ctx = ctx().await;
        let gate = Gate::default();
        let parts = parts();
        let admin = User(&["admin"]);

        assert!(
            !gate
                .allows("authenticated", check(&ctx, &parts, None))
                .await
                .unwrap()
        );
        assert!(
            gate.allows("authenticated", check(&ctx, &parts, Some(&admin)))
                .await
                .unwrap()
        );
        assert!(
            gate.allows("role:admin", check(&ctx, &parts, Some(&admin)))
                .await
                .unwrap()
        );
        assert!(
            !gate
                .allows("role:editor", check(&ctx, &parts, Some(&admin)))
                .await
                .unwrap()
        );
        assert!(
            !gate
                .allows("unknown.ability", check(&ctx, &parts, Some(&admin)))
                .await
                .unwrap()
        );
        assert!(gate.has("role:x") && gate.has("authenticated") && !gate.has("thing.do"));
    }

    #[tokio::test]
    async fn policies_decide() {
        let ctx = ctx().await;
        let parts = parts();
        let mut gate = Gate::default();
        gate.define(Always(false));
        assert!(
            !gate
                .allows(
                    "thing.do",
                    Check {
                        ctx: &ctx,
                        actor: None,
                        parts: &parts
                    }
                )
                .await
                .unwrap()
        );
        gate.define(Always(true));
        assert!(
            gate.allows(
                "thing.do",
                Check {
                    ctx: &ctx,
                    actor: None,
                    parts: &parts
                }
            )
            .await
            .unwrap()
        );
        assert_eq!(gate.abilities().collect::<Vec<_>>(), ["thing.do"]);
    }
}
