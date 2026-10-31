//! End-to-end behaviour of the default middleware stack and error rendering.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::result_large_err)]

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    routing::get,
};
use http_body_util::BodyExt;
use rustali_core::{App, AppContext, Config, Error, Result, middleware::REQUEST_ID_HEADER};
use tower::ServiceExt;

async fn panics() -> &'static str {
    panic!("handler panicked")
}

fn app(debug: bool) -> Router {
    let mut config = Config::default();
    config.app.debug = debug;

    let handlers: Router<AppContext> = Router::new()
        .route("/", get(|| async { "hello" }))
        .route("/api/ping", get(|| async { "pong" }))
        .route(
            "/bad",
            get(|| async { Err::<(), Error>(Error::bad_request("missing `q`")) }),
        )
        .route(
            "/boom",
            get(|| async {
                Err::<(), Error>(Error::internal(std::io::Error::other("database exploded")))
            }),
        )
        .route("/panic", get(panics));

    let (_, router) = App::builder()
        .config(config)
        .routes(handlers)
        .build()
        .expect("app builds");
    router
}

async fn send(
    router: Router,
    uri: &str,
    accept: Option<&str>,
) -> (StatusCode, http::HeaderMap, String) {
    let mut request = Request::builder().uri(uri);
    if let Some(accept) = accept {
        request = request.header(header::ACCEPT, accept);
    }
    let response = router
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn successful_responses_pass_through_with_request_id() {
    let (status, headers, body) = send(app(false), "/", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "hello");
    let id = headers.get(REQUEST_ID_HEADER).expect("request id header");
    assert_eq!(id.to_str().unwrap().len(), 36, "uuid v4");
}

#[tokio::test]
async fn unknown_route_renders_html_for_browsers() {
    let (status, headers, body) = send(app(false), "/missing", Some("text/html,*/*;q=0.8")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(
        headers[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert!(body.contains("<h1>404</h1>"));
    assert!(body.contains("Not Found"));
}

#[tokio::test]
async fn unknown_api_route_renders_problem_json() {
    let (status, headers, body) = send(app(false), "/api/missing", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(headers[header::CONTENT_TYPE], "application/problem+json");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    insta::assert_json_snapshot!(json, { ".request_id" => "[request-id]" });
}

#[tokio::test]
async fn bad_request_detail_is_public() {
    let (status, _, body) = send(app(false), "/bad", Some("application/json")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    insta::assert_json_snapshot!(json, { ".request_id" => "[request-id]" });
}

#[tokio::test]
async fn internal_error_hides_detail_unless_debug() {
    let (status, _, body) = send(app(false), "/boom", Some("application/json")).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(
        json.get("detail").is_none(),
        "production must not leak: {body}"
    );

    let (_, _, body) = send(app(true), "/boom", Some("application/json")).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["detail"], "database exploded");
}

#[tokio::test]
async fn panics_become_500_instead_of_dropping_the_connection() {
    let (status, _, _) = send(app(false), "/panic", None).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn build_fails_cleanly_on_invalid_config() {
    figment::Jail::expect_with(|jail| {
        jail.create_dir("config")?;
        jail.create_file("config/server.toml", "port = \"not-a-number\"")?;
        let result: Result<_> = App::builder().build();
        assert!(matches!(result, Err(Error::Config(_))));
        Ok(())
    });
}
