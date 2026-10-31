//! # Rustali
//!
//! A batteries-included, full-stack web framework for Rust. Add this one crate and
//! get HTTP routing, an ORM with code-first migrations, type-safe views, auth,
//! background jobs, mail and a CLI that all fit together.
//!
//! Most programs only need the prelude:
//!
//! ```no_run
//! use rustali::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     App::builder().controllers().run().await
//! }
//! ```

pub use rustali_core::{
    App, AppBuilder, AppContext, Command, CommandFuture, Config, Ctx, Error, ErrorBody, Inject,
    Provider, RegisterFuture, Registry, Result, Services, auth, command, config, middleware,
    router, services, telemetry,
};

#[cfg(feature = "macros")]
pub use rustali_macros::{Entity, controller};

/// Data layer: entities, queries, pagination, the `Db` extractor.
#[cfg(feature = "orm")]
pub mod orm {
    pub use rustali_orm::*;
}

/// Database migrations: runner, console commands, migration prelude.
#[cfg(feature = "orm")]
pub mod migrate {
    pub use rustali_migrate::*;
}

/// Re-exports of the underlying HTTP stack for advanced use.
pub mod http {
    pub use axum::{Router, extract, http::*, response, routing};
}

/// Everything a typical application file imports.
pub mod prelude {
    pub use axum::{
        Json,
        extract::{Form, Path, Query, State},
        response::{Html, IntoResponse, Redirect},
    };

    pub use crate::{
        App, AppContext, Config, Ctx, Error, Inject, Result,
        auth::{Actor, AuthFuture, AuthorizeLayer, Check, Gate, Policy},
        middleware::{Cors, Throttle},
        router::Controller,
    };

    #[cfg(feature = "macros")]
    pub use crate::{Entity, controller};

    #[cfg(feature = "orm")]
    pub use crate::{
        migrate::MigrateAppExt,
        // `orm::Query` is reachable as `Todo::query()`; the name stays free for axum's
        // `Query<Params>` extractor, which handlers use far more often.
        orm::{DatabaseProvider, Db, DbExt, DbResultExt, Page, prelude::*},
    };
}

/// Paths used by generated code. Not part of the public API; may change at any time.
#[doc(hidden)]
pub mod __private {
    pub use rustali_core::__private::*;
}
