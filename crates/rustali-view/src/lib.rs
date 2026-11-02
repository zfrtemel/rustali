//! View layer for the Rustali web framework.
//!
//! Two approaches, one trait. By default views are **Askama** templates checked at
//! compile time (a missing variable is a build error, not a 500); enable the `tera` feature
//! for **Tera** templates rendered at runtime with hot reload. Either way a handler returns a
//! [`View`], which renders to HTML and, for an `Accept: application/json` request, can fall
//! back to JSON when the template struct is serialisable — the same controller serving SSR
//! and API.
//!
//! ## Askama (default)
//!
//! ```ignore
//! use rustali::prelude::*;
//!
//! #[derive(Template)]
//! #[template(path = "posts/index.html")]
//! struct PostsIndex { posts: Vec<Post> }
//!
//! async fn index(db: Db) -> Result<View<PostsIndex>> {
//!     Ok(View::new(PostsIndex { posts: Post::all(&db).await? }))
//! }
//! ```
//!
//! ## Tera (feature = "tera")
//!
//! ```ignore
//! async fn index(views: Inject<Tera>) -> Result<Html<String>> {
//!     views.render("posts/index.html", &context! { posts => posts })
//! }
//! ```

mod response;

#[cfg(feature = "askama")]
pub use askama;
pub use response::{View, ViewError};

#[cfg(feature = "tera")]
pub mod tera_backend;
#[cfg(feature = "tera")]
pub use tera_backend::{TeraConfig, TeraProvider, TeraView};
