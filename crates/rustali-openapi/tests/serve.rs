//! End-to-end: a tiny app with one controller, serving the generated spec and Swagger UI.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rustali_core::{App, router::RouteDef};
use rustali_openapi::openapi_router;
use rustali_test::{TestClient, test_config};

/// Build an app that describes one route and mounts the OpenAPI router.
async fn client() -> TestClient {
    // `describe` adds route metadata without needing the controller macro here.
    let route_defs = [
        RouteDef {
            method: "GET",
            path: "/posts",
            name: Some("posts.index"),
            handler: "PostController::index",
            middleware: &[],
            authorize: None,
        },
        RouteDef {
            method: "GET",
            path: "/posts/{id}",
            name: None,
            handler: "PostController::show",
            middleware: &[],
            authorize: Some("post.view"),
        },
    ];
    let (_ctx, router) = App::builder()
        .config(test_config())
        .describe(route_defs)
        .routes(openapi_router())
        .build()
        .await
        .unwrap();
    TestClient::new(router)
}

#[tokio::test]
async fn openapi_json_lists_the_routes() {
    let client = client().await;
    let res = client.get("/openapi.json").await;
    res.assert_ok();
    let json = res.json();

    assert_eq!(json["openapi"], "3.1.0");
    assert_eq!(json["paths"]["/posts"]["get"]["operationId"], "posts.index");
    assert_eq!(
        json["paths"]["/posts/{id}"]["get"]["parameters"][0]["name"],
        "id"
    );
    // authorize shows up as a description note
    assert!(
        json["paths"]["/posts/{id}"]["get"]["description"]
            .as_str()
            .unwrap()
            .contains("post.view")
    );
}

#[tokio::test]
async fn swagger_ui_and_assets_are_served() {
    let client = client().await;

    client
        .get("/swagger")
        .await
        .assert_ok()
        .assert_see("swagger-ui");
    client
        .get("/swagger/swagger-ui-bundle.js")
        .await
        .assert_ok();
    client.get("/swagger/swagger-ui.css").await.assert_ok();
}
