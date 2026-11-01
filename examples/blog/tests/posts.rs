//! Integration test for the blog, driven by the framework's own `TestClient`.
//!
//! It builds the real application (same providers, routes and middleware as `main`),
//! points it at a throwaway SQLite file, runs migrations, then exercises the
//! server-rendered flow: the form renders with a CSRF token, an invalid submission
//! redirects back with errors, and a valid one creates a post and flashes success.

#![allow(clippy::unwrap_used)]

use blog::{migrations, models::Post};
use rustali::{
    http::Router,
    migrate::MigrateAppExt,
    orm::DbExt,
    prelude::*,
    test::{TestClient, test_config},
};

fn config(db_path: &str) -> Config {
    let mut cfg = test_config();
    cfg.extra.insert(
        "database".into(),
        toml::value::Table::from_iter([(
            "url".to_owned(),
            format!("sqlite://{db_path}?mode=rwc").into(),
        )])
        .into(),
    );
    cfg
}

async fn build(db_path: &str) -> (AppContext, Router) {
    let (ctx, router) = App::builder()
        .config(config(db_path))
        .provider(DatabaseProvider)
        .provider(SessionProvider)
        .migrations::<migrations::Migrator>()
        .controllers()
        .configure(rustali::session::session_layers)
        .build()
        .await
        .unwrap();
    let runner = ctx
        .service::<rustali::migrate::runner::MigrationRunner>()
        .unwrap();
    runner.0.up(ctx.db().unwrap().conn(), None).await.unwrap();
    (ctx, router)
}

fn token(html: &str) -> String {
    let marker = "name=\"_token\" value=\"";
    let start = html.find(marker).expect("csrf token in form") + marker.len();
    let end = html[start..].find('"').unwrap() + start;
    html[start..end].to_owned()
}

fn temp_db(tag: &str) -> (std::path::PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("rustali-blog-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("blog.db");
    let s = path.to_string_lossy().replace('\\', "/");
    (dir, s)
}

#[tokio::test]
async fn full_post_flow() {
    let (dir, db) = temp_db("flow");
    let (ctx, router) = build(&db).await;
    let client = TestClient::new(router);

    let home = client.get("/").await;
    home.assert_ok()
        .assert_see("Posts")
        .assert_see("No posts yet.");
    let tok = token(&home.text());

    // Invalid: blank body -> redirect back, errors + old input on re-render.
    client
        .post_form("/posts", &[("_token", &tok), ("title", "Hi"), ("body", "")])
        .await
        .assert_redirect(Some("/"));
    let after_invalid = client.get("/").await;
    after_invalid
        .assert_see("between 3 and 200")
        .assert_see("value=\"Hi\"");

    // Valid: creates the post, flashes success.
    let tok = token(&after_invalid.text());
    client
        .post_form(
            "/posts",
            &[
                ("_token", &tok),
                ("title", "First real post"),
                ("body", "Rustali works end to end."),
            ],
        )
        .await
        .assert_redirect(Some("/"));
    client
        .get("/")
        .await
        .assert_see("Post published.")
        .assert_see("First real post");

    assert_eq!(Post::count(ctx.db().unwrap().conn()).await.unwrap(), 1);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn missing_csrf_token_is_forbidden() {
    let (dir, db) = temp_db("csrf");
    let (_ctx, router) = build(&db).await;
    let client = TestClient::new(router);
    client.get("/").await.assert_ok(); // establish a session
    client
        .post_form("/posts", &[("title", "No token"), ("body", "nope")])
        .await
        .assert_status(rustali::http::StatusCode::FORBIDDEN);
    std::fs::remove_dir_all(&dir).ok();
}
