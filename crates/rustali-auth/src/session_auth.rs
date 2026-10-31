//! Session-backed login state and the handler-facing [`Auth`] actions.

use axum::http::request::Parts;
use rustali_core::{Error, Result};
use rustali_session::Session;

use crate::{Authenticatable, guard::SESSION_USER_KEY, password};

/// Read the logged-in user id from the request's session, if present.
pub(crate) async fn current_id(parts: &Parts) -> Option<String> {
    let session = parts.extensions.get::<Session>()?;
    session.get::<String>(SESSION_USER_KEY).await.ok().flatten()
}

/// Login/logout actions over a request's [`Session`]. Obtain it in a handler with the
/// [`Auth`](crate::extract::Auth) extractor's companion, or construct from a session.
#[derive(Debug, Clone)]
pub struct SessionAuth {
    session: Session,
}

impl SessionAuth {
    /// Wrap a session.
    #[must_use]
    pub const fn new(session: Session) -> Self {
        Self { session }
    }

    /// Record `user` as logged in and rotate the session id (prevents fixation).
    pub async fn login<U: Authenticatable>(&self, user: &U) -> Result<()> {
        self.session.cycle_id().await.map_err(Error::internal)?;
        self.session
            .insert(SESSION_USER_KEY, user.auth_id())
            .await
            .map_err(Error::internal)
    }

    /// Verify `password` against `user` and log in on success.
    pub async fn attempt<U: Authenticatable>(&self, user: &U, password: &str) -> Result<bool> {
        if password::verify(password, user.password_hash())? {
            self.login(user).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Log out and clear the whole session.
    pub async fn logout(&self) -> Result<()> {
        self.session.flush().await.map_err(Error::internal)
    }

    /// The currently stored user id, if any.
    pub async fn id(&self) -> Option<String> {
        self.session
            .get::<String>(SESSION_USER_KEY)
            .await
            .ok()
            .flatten()
    }
}
