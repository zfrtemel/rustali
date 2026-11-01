//! The in-process [`TestClient`].

use std::sync::Mutex;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, header},
};
use tower::ServiceExt;

use crate::response::TestResponse;

/// Drives a [`Router`] in-process, keeping cookies across requests.
///
/// Cloneable is deliberately not derived: the cookie jar is shared state scoped to one
/// logical session. Create a fresh client for an independent session.
pub struct TestClient {
    router: Router,
    cookies: Mutex<Vec<(String, String)>>,
}

impl TestClient {
    /// Wrap a built router (from `App::builder().build()`).
    #[must_use]
    pub const fn new(router: Router) -> Self {
        Self {
            router,
            cookies: Mutex::new(Vec::new()),
        }
    }

    /// `GET path`.
    pub async fn get(&self, path: &str) -> TestResponse {
        self.send(Method::GET, path, None, &[]).await
    }

    /// `DELETE path`.
    pub async fn delete(&self, path: &str) -> TestResponse {
        self.send(Method::DELETE, path, None, &[]).await
    }

    /// `POST path` with a JSON body.
    pub async fn post_json<T: serde::Serialize + Sync>(
        &self,
        path: &str,
        body: &T,
    ) -> TestResponse {
        let json = serde_json::to_vec(body).unwrap_or_default();
        self.send(
            Method::POST,
            path,
            Some(json),
            &[
                (header::CONTENT_TYPE.as_str(), "application/json"),
                (header::ACCEPT.as_str(), "application/json"),
            ],
        )
        .await
    }

    /// `PATCH path` with a JSON body.
    pub async fn patch_json<T: serde::Serialize + Sync>(
        &self,
        path: &str,
        body: &T,
    ) -> TestResponse {
        let json = serde_json::to_vec(body).unwrap_or_default();
        self.send(
            Method::PATCH,
            path,
            Some(json),
            &[
                (header::CONTENT_TYPE.as_str(), "application/json"),
                (header::ACCEPT.as_str(), "application/json"),
            ],
        )
        .await
    }

    /// `POST path` with a urlencoded form body (HTML form submission).
    pub async fn post_form(&self, path: &str, fields: &[(&str, &str)]) -> TestResponse {
        let body = serde_urlencoded::to_string(fields).unwrap_or_default();
        self.send(
            Method::POST,
            path,
            Some(body.into_bytes()),
            &[
                (
                    header::CONTENT_TYPE.as_str(),
                    "application/x-www-form-urlencoded",
                ),
                (header::ACCEPT.as_str(), "text/html"),
            ],
        )
        .await
    }

    /// Send an arbitrary request, applying stored cookies and capturing any `Set-Cookie`.
    pub async fn send(
        &self,
        method: Method,
        path: &str,
        body: Option<Vec<u8>>,
        headers: &[(&str, &str)],
    ) -> TestResponse {
        let mut builder = Request::builder().method(method).uri(path);
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        if let Some(cookie) = self.cookie_header() {
            builder = builder.header(header::COOKIE, cookie);
        }
        let request = builder
            .body(body.map_or_else(Body::empty, Body::from))
            .unwrap_or_else(|_| Request::new(Body::empty()));

        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .unwrap_or_else(|never| match never {});

        self.store_cookies(&response);
        TestResponse::from_response(response).await
    }

    fn cookie_header(&self) -> Option<String> {
        let cookies = self
            .cookies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if cookies.is_empty() {
            return None;
        }
        Some(
            cookies
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("; "),
        )
    }

    fn store_cookies(&self, response: &axum::response::Response) {
        let mut jar = self
            .cookies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for value in response.headers().get_all(header::SET_COOKIE) {
            let Ok(text) = value.to_str() else { continue };
            let Some(pair) = text.split(';').next() else {
                continue;
            };
            let Some((name, val)) = pair.split_once('=') else {
                continue;
            };
            let (name, val) = (name.trim().to_owned(), val.trim().to_owned());
            jar.retain(|(k, _)| k != &name);
            jar.push((name, val));
        }
    }
}
