//! Source templates for `make:*`. Minijinja syntax (`{{ name }}`).
//!
//! Kept as string constants so the CLI is a single self-contained binary. Raw-string
//! hashes are kept uniform across every template for readability, not minimised.
#![allow(clippy::needless_raw_string_hashes)]

pub(crate) const CONTROLLER: &str = r#"//! The {{ name }} controller.

use rustali::prelude::*;

pub struct {{ name }}Controller;

#[controller("/{{ module }}")]
impl {{ name }}Controller {
    #[get("/", name = "{{ module }}.index")]
    async fn index() -> Result<&'static str> {
        Ok("{{ name }} index")
    }
}
"#;

pub(crate) const MODEL: &str = r#"//! The {{ name }} entity.

use rustali::{
    orm::chrono::{DateTime, Utc},
    prelude::*,
};
use serde::Serialize;

#[derive(Entity, Debug, Clone, Serialize)]
#[table("{{ table }}")]
pub struct {{ name }} {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
"#;

pub(crate) const MIGRATION: &str = r#"//! Migration.

#![allow(elided_lifetimes_in_paths)]

use rustali::migrate::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table({{ table | capitalize }}::Table)
                    .if_not_exists()
                    .col(ColumnDef::new({{ table | capitalize }}::Id).big_integer().not_null().auto_increment().primary_key())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table({{ table | capitalize }}::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum {{ table | capitalize }} {
    Table,
    Id,
}
"#;

pub(crate) const JOB: &str = r#"//! The {{ name }} background job.

use rustali::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct {{ name }} {
    // fields the job needs, e.g. `pub user_id: i64,`
}

#[async_trait]
impl Job for {{ name }} {
    const NAME: &'static str = "{{ job_name }}";

    async fn handle(&self, _ctx: &AppContext) -> Result<()> {
        // do the work
        Ok(())
    }
}
"#;

pub(crate) const MAIL: &str = r#"//! The {{ name }} mailable.

use rustali::prelude::*;

pub struct {{ name }} {
    // data the email needs
}

impl Mailable for {{ name }} {
    fn subject(&self) -> String {
        "{{ name }}".into()
    }

    fn html(&self) -> String {
        "<h1>{{ name }}</h1>".into()
    }
}
"#;

pub(crate) const MIDDLEWARE: &str = r#"//! The {{ name }} middleware (a tower layer via axum's from_fn).

use rustali::http::{extract::Request, middleware::Next, response::Response};

/// Wrap this with `axum::middleware::from_fn({{ name | lower }})` and add it through
/// `App::builder().layer(...)` or a controller's `#[middleware(...)]`.
pub async fn {{ name | lower }}(request: Request, next: Next) -> Response {
    // inspect or modify the request here
    next.run(request).await
}
"#;

pub(crate) const POLICY: &str = r#"//! The {{ name }} authorization policy.

use rustali::prelude::*;

pub struct {{ name }};

impl Policy for {{ name }} {
    fn ability(&self) -> &'static str {
        "{{ ability }}"
    }

    fn allows<'a>(&'a self, check: Check<'a>) -> AuthFuture<'a, bool> {
        Box::pin(async move {
            // decide based on check.actor / check.parts
            Ok(check.actor.is_some())
        })
    }
}
"#;

pub(crate) const TEST: &str = r#"//! Integration test: {{ name }}.

use rustali::test::{TestClient, test_config};
use rustali::prelude::*;

#[tokio::test]
async fn {{ name }}_works() {
    let (_ctx, router) = App::builder()
        .config(test_config())
        .controllers()
        .build()
        .await
        .expect("app builds");
    let client = TestClient::new(router);
    client.get("/").await.assert_ok();
}
"#;
