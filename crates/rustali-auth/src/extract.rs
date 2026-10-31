//! Handler extractors: [`Auth`], [`OptionalAuth`], [`Guest`] and [`Login`].

use std::{future::ready, marker::PhantomData, ops::Deref};

use axum::{
    extract::{FromRef, FromRequestParts, OptionalFromRequestParts},
    http::request::Parts,
};
use rustali_core::{AppContext, Error, Result, auth::resolve_actor};
use rustali_session::Session;

use crate::{Authenticatable, session_auth::SessionAuth};

/// Resolve the current request's user as the concrete type `U`.
async fn current_user<U: Authenticatable>(ctx: &AppContext, parts: &Parts) -> Option<U> {
    let actor = resolve_actor(ctx, parts).await.ok().flatten()?;
    actor.as_any().downcast_ref::<U>().cloned()
}

/// The authenticated user, or a 401 (API) / handled error (SSR) when absent.
///
/// `U` is the application's [`Authenticatable`] type. Resolution runs the configured
/// guard chain; the loaded actor is downcast back to `U`.
///
/// ```ignore
/// async fn dashboard(Auth(user): Auth<User>) -> Result<View<Dashboard>> { .. }
/// ```
#[derive(Debug, Clone)]
pub struct Auth<U>(pub U);

impl<U> Deref for Auth<U> {
    type Target = U;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<U> Auth<U> {
    /// The authenticated user.
    pub const fn user(&self) -> &U {
        &self.0
    }
}

impl<U, S> FromRequestParts<S> for Auth<U>
where
    U: Authenticatable,
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ctx = AppContext::from_ref(state);
        current_user::<U>(&ctx, parts)
            .await
            .map(Auth)
            .ok_or(Error::Unauthorized)
    }
}

/// The authenticated user if present, `None` otherwise (never fails).
#[derive(Debug, Clone)]
pub struct OptionalAuth<U>(pub Option<U>);

impl<U> Deref for OptionalAuth<U> {
    type Target = Option<U>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<U, S> FromRequestParts<S> for OptionalAuth<U>
where
    U: Authenticatable,
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ctx = AppContext::from_ref(state);
        Ok(Self(current_user::<U>(&ctx, parts).await))
    }
}

/// Also usable as `OptionalAuth<U>` through axum's `Option<_>` extractor.
impl<U, S> OptionalFromRequestParts<S> for OptionalAuth<U>
where
    U: Authenticatable,
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        let ctx = AppContext::from_ref(state);
        Ok(Some(Self(current_user::<U>(&ctx, parts).await)))
    }
}

/// Rejects authenticated users (e.g. the login page should 403 if already logged in).
pub struct Guest<U>(PhantomData<fn() -> U>);

impl<U, S> FromRequestParts<S> for Guest<U>
where
    U: Authenticatable,
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ctx = AppContext::from_ref(state);
        if current_user::<U>(&ctx, parts).await.is_some() {
            Err(Error::Forbidden)
        } else {
            Ok(Self(PhantomData))
        }
    }
}

/// Login/logout actions bound to the request session.
///
/// ```ignore
/// async fn do_login(login: Login<User>, Form(creds): Form<Credentials>) -> Result<Redirect> {
///     let user = Users.by_email(&creds.email).await?.ok_or(Error::Unauthorized)?;
///     if login.attempt(&user, &creds.password).await? {
///         Ok(Redirect::to("/dashboard"))
///     } else {
///         Err(Error::Unauthorized)
///     }
/// }
/// ```
pub struct Login<U> {
    session: SessionAuth,
    _marker: PhantomData<fn() -> U>,
}

impl<U: Authenticatable> Login<U> {
    /// Verify a password and log the user in on success.
    pub async fn attempt(&self, user: &U, password: &str) -> Result<bool> {
        self.session.attempt(user, password).await
    }

    /// Log a user in without checking a password (after registration, `OAuth`, ...).
    pub async fn login(&self, user: &U) -> Result<()> {
        self.session.login(user).await
    }

    /// Log out.
    pub async fn logout(&self) -> Result<()> {
        self.session.logout().await
    }
}

impl<U, S> FromRequestParts<S> for Login<U>
where
    U: Authenticatable,
    S: Send + Sync,
{
    type Rejection = Error;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let session = parts.extensions.get::<Session>().cloned();
        ready(session.map_or_else(
            || Err(Error::internal(NoSession)),
            |session| {
                Ok(Self {
                    session: SessionAuth::new(session),
                    _marker: PhantomData,
                })
            },
        ))
    }
}

/// Raised when login actions are used without the session layer.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("no session; add SessionProvider before AuthProvider")]
pub struct NoSession;
