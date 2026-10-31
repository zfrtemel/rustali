//! Request extractors provided by the kernel.

use std::{
    convert::Infallible,
    future::{Future, ready},
    ops::Deref,
    sync::Arc,
};

use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};

use crate::{AppContext, Error};

/// Extracts the shared [`AppContext`].
///
/// Equivalent to `State<AppContext>` but reads better in handler signatures and
/// dereferences straight to the context:
///
/// ```ignore
/// async fn home(ctx: Ctx) -> String {
///     ctx.config().app.name.clone()
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Ctx(pub AppContext);

impl<S> FromRequestParts<S> for Ctx
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Infallible;

    fn from_request_parts(
        _parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        ready(Ok(Self(AppContext::from_ref(state))))
    }
}

impl Deref for Ctx {
    type Target = AppContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Extracts a registered service by type (.NET-style constructor injection, but per
/// handler). Fails with a 500 and a clear message when the service is missing.
///
/// ```ignore
/// async fn index(Inject(mailer): Inject<Mailer>) -> Result<()> {
///     mailer.send(..).await
/// }
/// ```
#[derive(Debug)]
pub struct Inject<T>(pub Arc<T>);

impl<T, S> FromRequestParts<S> for Inject<T>
where
    T: Send + Sync + 'static,
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    fn from_request_parts(
        _parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        ready(
            AppContext::from_ref(state)
                .services()
                .require::<T>()
                .map(Self),
        )
    }
}

impl<T> Deref for Inject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Clone for Inject<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
