//! `#[derive(Validate)]` behaviour and the `Validated` extractors.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::{
    Form, Json, Router,
    body::Body,
    http::{Request, StatusCode, header},
    routing::post,
};
use http_body_util::BodyExt;
use rustali_core::{App, AppContext, Config};
use rustali_macros::Validate;
use rustali_validate::{Validate, Validated};
use serde::Deserialize;
use tower::ServiceExt;

#[allow(clippy::ptr_arg)] // custom-rule signature is fn(&T) where the field is String
fn no_spam(value: &String) -> Result<(), String> {
    if value.to_lowercase().contains("spam") {
        Err("No spam, please.".to_owned())
    } else {
        Ok(())
    }
}

#[derive(Debug, Deserialize, Validate)]
#[validate(crate = "rustali_validate")]
struct Address {
    #[rule(required)]
    city: String,
}

#[derive(Debug, Deserialize, Validate)]
#[validate(crate = "rustali_validate")]
struct CreatePost {
    #[rule(required, length(min = 3, max = 10))]
    title: String,
    #[rule(email)]
    email: Option<String>,
    #[rule(range(min = 1, max = 5))]
    rating: u8,
    #[rule(one_of("draft", "published"))]
    status: String,
    #[rule(confirmed)]
    password: String,
    password_confirmation: String,
    #[rule(custom = "no_spam")]
    body: String,
    #[rule(nested)]
    address: Address,
}

fn valid() -> CreatePost {
    CreatePost {
        title: "Hello".into(),
        email: Some("a@b.co".into()),
        rating: 3,
        status: "draft".into(),
        password: "secret".into(),
        password_confirmation: "secret".into(),
        body: "nice".into(),
        address: Address {
            city: "Ankara".into(),
        },
    }
}

#[test]
fn valid_input_passes() {
    assert!(valid().validate().is_ok());
}

#[test]
fn every_failure_is_collected() {
    let input = CreatePost {
        title: "  ".into(),
        email: Some("nope".into()),
        rating: 9,
        status: "weird".into(),
        password: "a".into(),
        password_confirmation: "b".into(),
        body: "buy SPAM".into(),
        address: Address {
            city: String::new(),
        },
    };
    let errors = input.validate().unwrap_err();
    // A blank title fails both `required` and `length(min = 3)`; all rules are collected.
    assert_eq!(
        errors.get("title"),
        [
            "The title field is required.",
            "The title must be between 3 and 10 characters.",
        ]
    );
    assert_eq!(
        errors.first("email"),
        Some("The email must be a valid email address.")
    );
    assert_eq!(
        errors.first("rating"),
        Some("The rating must be between 1 and 5.")
    );
    assert!(
        errors
            .first("status")
            .unwrap()
            .starts_with("The selected status is invalid")
    );
    assert_eq!(
        errors.first("password"),
        Some("The password confirmation does not match.")
    );
    assert_eq!(errors.first("body"), Some("No spam, please."));
    // The key is prefixed with the parent field; the message keeps the child's own
    // field name.
    assert_eq!(
        errors.first("address.city"),
        Some("The city field is required.")
    );
    assert_eq!(errors.len(), 8);
}

#[test]
fn optional_fields_skip_rules_when_absent_and_length_is_checked() {
    let mut input = valid();
    input.email = None;
    assert!(input.validate().is_ok());
    input.title = "way too long title".into();
    let errors = input.validate().unwrap_err();
    assert_eq!(
        errors.first("title"),
        Some("The title must be between 3 and 10 characters.")
    );
}

// --- extractors ----------------------------------------------------------------------

#[derive(Debug, Deserialize, Validate)]
#[validate(crate = "rustali_validate")]
struct Signup {
    #[rule(required, length(min = 2, max = 20))]
    name: String,
}

async fn json_handler(Validated(Json(input)): Validated<Json<Signup>>) -> String {
    format!("hi {}", input.name)
}

async fn form_handler(input: Validated<Form<Signup>>) -> String {
    format!("hi {}", input.into_inner().name)
}

async fn app() -> axum::Router {
    let routes: Router<AppContext> = Router::new()
        .route("/api/signup", post(json_handler))
        .route("/signup", post(form_handler));
    App::builder()
        .config(Config::default())
        .routes(routes)
        .build()
        .await
        .unwrap()
        .1
}

async fn send(uri: &str, content_type: &str, body: &str, accept: &str) -> (StatusCode, String) {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::ACCEPT, accept)
        .body(Body::from(body.to_owned()))
        .unwrap();
    let response = app().await.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

#[tokio::test]
async fn json_extractor_validates() {
    let (status, body) = send(
        "/api/signup",
        "application/json",
        r#"{"name":"Zafer"}"#,
        "application/json",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "hi Zafer");

    let (status, body) = send(
        "/api/signup",
        "application/json",
        r#"{"name":""}"#,
        "application/json",
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let doc: serde_json::Value = serde_json::from_str(&body).unwrap();
    // The fixed { success, data, error } envelope; field errors live under
    // error.details.fields.
    assert_eq!(doc["success"], false);
    assert_eq!(doc["error"]["code"], 422);
    assert_eq!(
        doc["error"]["details"]["fields"]["name"][0],
        "The name field is required."
    );

    let (status, _) = send(
        "/api/signup",
        "application/json",
        "{not json",
        "application/json",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn form_extractor_without_session_renders_422() {
    let (status, body) = send(
        "/signup",
        "application/x-www-form-urlencoded",
        "name=Z",
        "text/html",
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body.contains("422"));

    let (status, body) = send(
        "/signup",
        "application/x-www-form-urlencoded",
        "name=Zafer",
        "text/html",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "hi Zafer");
}
