//! End-to-end behaviour of the default middleware stack, error rendering and DI.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    routing::get,
};
use http_body_util::BodyExt;
use rustali_core::{
    App, AppContext, Config, Error, Inject, Provider, RegisterFuture, Registry, Result,
    middleware::REQUEST_ID_HEADER,
};
use tower::ServiceExt;

/// A service registered by a provider, to exercise `Inject`.
struct Greeting(&'static str);

struct GreetingProvider;

impl Provider for GreetingProvider {
    fn register<'a>(&'a self, registry: &'a mut Registry<'_>) -> RegisterFuture<'a> {
        Box::pin(async move {
            registry.insert(Greeting("hello from provider"));
            Ok(())
        })
    }
}

struct Unregistered;

async fn panics() -> &'static str {
    panic!("handler panicked")
}

async fn greet(Inject(greeting): Inject<Greeting>) -> &'static str {
    greeting.0
}

async fn missing(_svc: Inject<Unregistered>) -> &'static str {
    "unreachable"
}

async fn app(debug: bool) -> Router {
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
        .route("/panic", get(panics))
        .route("/greet", get(greet))
        .route("/missing-service", get(missing));

    let (_, router) = App::builder()
        .config(config)
        .provider(GreetingProvider)
        .routes(handlers)
        .build()
        .await
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
    let (status, headers, body) = send(app(false).await, "/", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "hello");
    let id = headers.get(REQUEST_ID_HEADER).expect("request id header");
    assert_eq!(id.to_str().unwrap().len(), 36, "uuid v4");
}

#[tokio::test]
async fn unknown_route_renders_html_for_browsers() {
    let (status, headers, body) =
        send(app(false).await, "/missing", Some("text/html,*/*;q=0.8")).await;
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
async fn unknown_api_route_renders_json_envelope() {
    let (status, headers, body) = send(app(false).await, "/api/missing", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(headers[header::CONTENT_TYPE], "application/json");
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    // The fixed { success, data, error } envelope.
    assert_eq!(json["success"], false);
    assert_eq!(json["data"], serde_json::Value::Null);
    assert_eq!(json["error"]["code"], 404);
    assert_eq!(json["error"]["message"], "Not Found");
    assert_eq!(json["error"]["details"]["instance"], "/api/missing");
}

#[tokio::test]
async fn bad_request_detail_is_public() {
    let (status, _, body) = send(app(false).await, "/bad", Some("application/json")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["success"], false);
    // A client error puts its public message in error.message.
    assert_eq!(json["error"]["code"], 400);
    assert!(
        !json["error"]["message"].as_str().unwrap().is_empty(),
        "bad request message is public: {body}"
    );
}

#[tokio::test]
async fn internal_error_hides_detail_unless_debug() {
    // In production the message is the generic title, never the internal detail.
    let (status, _, body) = send(app(false).await, "/boom", Some("application/json")).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["error"]["message"], "Internal Server Error");
    assert!(
        !body.contains("database exploded"),
        "production must not leak: {body}"
    );

    // With debug on, the real detail surfaces in the message.
    let (_, _, body) = send(app(true).await, "/boom", Some("application/json")).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["error"]["message"], "database exploded");
}

#[tokio::test]
async fn panics_become_500_instead_of_dropping_the_connection() {
    let (status, _, _) = send(app(false).await, "/panic", None).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn provider_services_are_injectable() {
    let (status, _, body) = send(app(false).await, "/greet", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "hello from provider");
}

#[tokio::test]
async fn missing_service_is_a_descriptive_500() {
    let (status, _, body) = send(
        app(true).await,
        "/missing-service",
        Some("application/json"),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(body.contains("Unregistered"), "{body}");
    assert!(body.contains("not registered"), "{body}");
}

#[tokio::test]
async fn provider_failure_aborts_build() {
    struct Failing;
    impl Provider for Failing {
        fn register<'a>(&'a self, _: &'a mut Registry<'_>) -> RegisterFuture<'a> {
            Box::pin(async { Err(Error::bad_request("no database url")) })
        }
    }
    let result: Result<_> = App::builder()
        .config(Config::default())
        .provider(Failing)
        .build()
        .await;
    assert!(matches!(result, Err(Error::BadRequest(_))));
}
