//! The [`Authenticatable`] trait and the actor adapter.
//!
//! An application's user type implements [`Authenticatable`]; it is usually a
//! `#[derive(Entity)]` struct, but the trait is deliberately storage-agnostic so a
//! guard can load users from anywhere. [`AuthUser`] wraps one as a
//! [`rustali_core::auth::Actor`] so `#[authorize]` and the `Gate` work unchanged.

use std::{any::Any, sync::Arc};

use rustali_core::auth::Actor;

/// A type that can be authenticated: it has a stable id, a stored password hash and a
/// set of roles.
pub trait Authenticatable: Clone + Send + Sync + 'static {
    /// Stable identifier, serialised into sessions and tokens.
    fn auth_id(&self) -> String;

    /// The argon2 PHC hash to verify passwords against.
    fn password_hash(&self) -> &str;

    /// Role names for the built-in `role:<name>` ability. Empty by default.
    fn roles(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Adapts any [`Authenticatable`] into the kernel's [`Actor`].
#[derive(Debug, Clone)]
pub struct AuthUser<U>(pub U);

impl<U: Authenticatable> Actor for AuthUser<U> {
    fn id(&self) -> String {
        self.0.auth_id()
    }

    fn roles(&self) -> Vec<String> {
        self.0.roles()
    }

    fn as_any(&self) -> &dyn Any {
        &self.0
    }
}

impl<U: Authenticatable> AuthUser<U> {
    /// Wrap a user as a shared actor.
    pub fn actor(user: U) -> Arc<dyn Actor> {
        Arc::new(Self(user))
    }
}
