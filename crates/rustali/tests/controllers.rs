//! `#[controller]` end to end: routing, controller/method middleware, authorization,
//! route metadata and auto-discovery.

// Test handlers are deliberately trivial (no `.await`) and undocumented.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::unused_async,
    clippy::unused_async_trait_impl,
    missing_docs
)]

use std::any::Any;

use http_body_util::BodyExt;
use rustali::{
    auth::{ActorRef, ActorResolver, ActorResolverHandle},
    http::{Request, StatusCode, header, request::Parts},
    prelude::*,
    router::RouteTable,
};
use serde::Serialize;
use tower::ServiceExt;

// --- a fake auth layer: the `x-user` header names the actor and their roles -------

struct HeaderUser {
    id: String,
    roles: Vec<String>,
}

impl Actor for HeaderUser {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn roles(&self) -> Vec<String> {
        self.roles.clone()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct HeaderResolver;

impl ActorResolver for HeaderResolver {
    fn resolve<'a>(
        &'a self,
        _: &'a AppContext,
        parts: &'a Parts,
    ) -> AuthFuture<'a, Option<ActorRef>> {
        Box::pin(async move {
            let Some(value) = parts.headers.get("x-user").and_then(|v| v.to_str().ok()) else {
                return Ok(None);
            };
            let (id, roles) = value.split_once(':').unwrap_or((value, ""));
            let actor: ActorRef = std::sync::Arc::new(HeaderUser {
                id: id.to_owned(),
                roles: roles
                    .split(',')
                    .filter(|r| !r.is_empty())
                    .map(str::to_owned)
                    .collect(),
            });
            Ok(Some(actor))
        })
    }
}

/// Only the owner (`x-user: 7`) may touch post 7.
struct OwnsPost;

impl Policy for OwnsPost {
    fn ability(&self) -> &'static str {
        "post.update"
    }
    fn allows<'a>(&'a self, check: Check<'a>) -> AuthFuture<'a, bool> {
        Box::pin(async move {
            let Some(actor) = check.actor else {
                return Ok(false);
            };
            Ok(check
                .parts
                .uri
                .path()
                .ends_with(&format!("/{}", actor.id())))
        })
    }
}

// --- controllers ---------------------------------------------------------------------

#[derive(Serialize)]
struct Post {
    id: i64,
    title: String,
}

pub struct PostController;

#[controller("/posts")]
#[middleware(Throttle::per_minute(1000))]
impl PostController {
    #[get("/", name = "posts.index")]
    async fn index() -> Json<Vec<Post>> {
        Json(vec![Post {
            id: 1,
            title: "hello".into(),
        }])
    }

    #[get("/{id}", name = "posts.show")]
    async fn show(Path(id): Path<i64>) -> Result<Json<Post>> {
        if id == 0 {
            return Err(Error::NotFound);
        }
        Ok(Json(Post {
            id,
            title: format!("post {id}"),
        }))
    }

    #[post("/")]
    #[authorize(role = "editor")]
    async fn store() -> (StatusCode, &'static str) {
        (StatusCode::CREATED, "created")
    }

    #[patch("/{id}")]
    #[authorize("post.update")]
    async fn update(Path(id): Path<i64>) -> String {
        format!("updated {id}")
    }

    #[delete("/{id}")]
    #[authorize("authenticated")]
    #[middleware(Throttle::per_minute(2))]
    async fn destroy(Path(id): Path<i64>) -> String {
        format!("deleted {id}")
    }
}

pub struct HealthController;

#[controller]
impl HealthController {
    #[get("/health")]
    async fn health() -> &'static str {
        "ok"
    }
}

// --- harness -------------------------------------------------------------------------

async fn app() -> (AppContext, axum::Router) {
    let mut config = Config::default();
    config.app.debug = true;
    App::builder()
        .config(config)
        .provide(ActorResolverHandle(Box::new(HeaderResolver)))
        .policy(OwnsPost)
        .controllers()
        .build()
        .await
        .expect("app builds")
}

async fn send(
    router: axum::Router,
    method: &str,
    uri: &str,
    user: Option<&str>,
) -> (StatusCode, rustali::http::HeaderMap, String) {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::ACCEPT, "application/json");
    if let Some(user) = user {
        request = request.header("x-user", user);
    }
    let response = router
        .oneshot(request.body(axum::body::Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn routes_are_mounted_with_prefix() {
    let (_, router) = app().await;
    let (status, _, body) = send(router.clone(), "GET", "/posts", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, r#"[{"id":1,"title":"hello"}]"#);

    let (status, _, body) = send(router.clone(), "GET", "/posts/5", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("post 5"));

    let (status, _, _) = send(router.clone(), "GET", "/posts/0", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _, body) = send(router, "GET", "/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "ok");
}

#[tokio::test]
async fn authorize_role_gives_401_then_403_then_201() {
    let (_, router) = app().await;
    let (status, _, _) = send(router.clone(), "POST", "/posts", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _, _) = send(router.clone(), "POST", "/posts", Some("1:viewer")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, body) = send(router, "POST", "/posts", Some("1:editor")).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body, "created");
}

#[tokio::test]
async fn policies_see_the_request() {
    let (_, router) = app().await;
    let (status, _, _) = send(router.clone(), "PATCH", "/posts/7", Some("3")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, body) = send(router, "PATCH", "/posts/7", Some("7")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "updated 7");
}

#[tokio::test]
async fn method_throttle_limits_and_sets_headers() {
    let (_, router) = app().await;
    let (status, headers, _) = send(router.clone(), "DELETE", "/posts/1", Some("9")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["x-ratelimit-remaining"], "1");
    let (status, _, _) = send(router.clone(), "DELETE", "/posts/2", Some("9")).await;
    assert_eq!(status, StatusCode::OK);
    let (status, headers, body) = send(router, "DELETE", "/posts/3", Some("9")).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert!(
        headers.contains_key(header::RETRY_AFTER),
        "Retry-After survives re-rendering"
    );
    // The fixed { success, data, error } envelope with the 429 inside error.
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["success"], false);
    assert_eq!(json["error"]["code"], 429);
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("too many requests"),
        "{body}"
    );
}

#[tokio::test]
async fn route_table_lists_everything() {
    let (ctx, _) = app().await;
    let table = ctx.service::<RouteTable>().unwrap();
    let rows: Vec<String> = table
        .routes()
        .iter()
        .map(|r| {
            format!(
                "{} {} {} {:?} {:?} {:?}",
                r.method, r.path, r.handler, r.name, r.middleware, r.authorize
            )
        })
        .collect();
    insta::assert_yaml_snapshot!(rows);
    assert_eq!(
        table.by_name("posts.show").map(|r| r.path),
        Some("/posts/{id}")
    );
    let rendered = table.render();
    assert!(rendered.contains("PostController::destroy"));
    assert!(rendered.contains("authorize(authenticated)"));
}

#[tokio::test]
async fn routes_command_is_registered() {
    let (ctx, _) = app().await;
    assert!(ctx.commands().get("routes").is_some());
}
