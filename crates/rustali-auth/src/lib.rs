//! Authentication for the Rustali web framework.
//!
//! Ships ready to use: argon2id password hashing, a storage-agnostic
//! [`Authenticatable`] user, a [`UserStore`] the application implements, and a guard
//! chain (cookie [`Session`](guard::Guard::Session) for SSR, `Bearer` JWT for APIs)
//! wired into the kernel's `Gate` so `#[authorize]` sees the current user.
//!
//! ```ignore
//! use rustali::prelude::*;
//!
//! #[derive(Entity, Clone)]
//! struct User {
//!     id: i64,
//!     #[column(unique)] email: String,
//!     password_hash: String,
//! }
//!
//! impl Authenticatable for User {
//!     fn auth_id(&self) -> String { self.id.to_string() }
//!     fn password_hash(&self) -> &str { &self.password_hash }
//! }
//!
//! struct Users;
//! impl UserStore for Users {
//!     type User = User;
//!     fn find_by_id<'a>(&'a self, ctx: &'a AppContext, id: &'a str) -> StoreFuture<'a, Option<User>> {
//!         Box::pin(async move { Ok(User::find(&ctx.db()?, id.parse().unwrap_or(0)).await?) })
//!     }
//!     fn find_by_login<'a>(&'a self, ctx: &'a AppContext, email: &'a str) -> StoreFuture<'a, Option<User>> {
//!         Box::pin(async move { User::query().filter(User::EMAIL.eq(email)).fetch_one(&ctx.db()?).await })
//!     }
//! }
//!
//! App::builder()
//!     .provider(DatabaseProvider)
//!     .provider(SessionProvider)
//!     .provider(AuthProvider::new(Users))
//!     .configure(session_layers)
//!     .controllers()
//!     .run().await
//! ```

pub mod extract;
pub mod guard;
pub mod password;
pub mod provider;
pub mod session_auth;
pub mod user;

#[cfg(feature = "jwt")]
pub mod jwt;

pub use extract::{Auth, Guest, Login, OptionalAuth};
pub use guard::Guard;
pub use provider::{AuthProvider, StoreFuture, UserStore};
pub use session_auth::SessionAuth;
pub use user::{AuthUser, Authenticatable};
