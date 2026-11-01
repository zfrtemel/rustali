//! The captured [`TestResponse`] and its fluent assertions.

use axum::http::{HeaderMap, StatusCode};

/// A fully-read response, ready for assertions. Every `assert_*` returns `&Self` so
/// they chain; failures panic with a helpful message (the point of a test helper).
pub struct TestResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl TestResponse {
    pub(crate) async fn from_response(response: axum::response::Response) -> Self {
        use http_body_util::BodyExt;
        let status = response.status();
        let headers = response.headers().clone();
        let body = response
            .into_body()
            .collect()
            .await
            .map(|c| c.to_bytes().to_vec())
            .unwrap_or_default();
        Self {
            status,
            headers,
            body,
        }
    }

    /// The status code.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        self.status
    }

    /// The body as UTF-8 (lossy).
    #[must_use]
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// The body parsed as JSON. Panics if it is not valid JSON.
    #[must_use]
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|err| panic!("response body is not JSON: {err}\nbody: {}", self.text()))
    }

    /// A response header value as a string, if present.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<String> {
        self.headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    }

    /// Assert an exact status.
    pub fn assert_status(&self, status: StatusCode) -> &Self {
        assert_eq!(
            self.status,
            status,
            "unexpected status\nbody: {}",
            self.text()
        );
        self
    }

    /// Assert `200 OK`.
    pub fn assert_ok(&self) -> &Self {
        self.assert_status(StatusCode::OK)
    }

    /// Assert a 3xx redirect, optionally to a specific location.
    pub fn assert_redirect(&self, to: Option<&str>) -> &Self {
        assert!(
            self.status.is_redirection(),
            "expected a redirect, got {}\nbody: {}",
            self.status,
            self.text()
        );
        if let Some(to) = to {
            assert_eq!(
                self.header("location").as_deref(),
                Some(to),
                "unexpected redirect target"
            );
        }
        self
    }

    /// Assert the body contains `needle` (substring).
    pub fn assert_see(&self, needle: &str) -> &Self {
        let text = self.text();
        assert!(
            text.contains(needle),
            "expected body to contain {needle:?}\nbody: {text}"
        );
        self
    }

    /// Assert the body does *not* contain `needle`.
    pub fn assert_dont_see(&self, needle: &str) -> &Self {
        let text = self.text();
        assert!(
            !text.contains(needle),
            "expected body NOT to contain {needle:?}\nbody: {text}"
        );
        self
    }

    /// Assert the JSON body has a value at `pointer` (RFC 6901, e.g. `/items/0/id`).
    pub fn assert_json_has(&self, pointer: &str) -> &Self {
        let json = self.json();
        assert!(
            json.pointer(pointer).is_some(),
            "missing JSON pointer {pointer:?} in {json}"
        );
        self
    }

    /// Assert the JSON value at `pointer` equals `expected`.
    pub fn assert_json_at(&self, pointer: &str, expected: &serde_json::Value) -> &Self {
        let json = self.json();
        let actual = json.pointer(pointer);
        assert_eq!(
            actual,
            Some(expected),
            "JSON at {pointer:?} mismatch in {json}"
        );
        self
    }

    /// Assert a header is present (and optionally equals `value`).
    pub fn assert_header(&self, name: &str, value: Option<&str>) -> &Self {
        match value {
            Some(expected) => assert_eq!(self.header(name).as_deref(), Some(expected)),
            None => assert!(self.header(name).is_some(), "missing header {name:?}"),
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use axum::{Router, routing::get};

    use crate::TestClient;

    #[tokio::test]
    async fn assertions_work_against_a_plain_router() {
        let app = Router::new()
            .route("/", get(|| async { "hello world" }))
            .route(
                "/api/n",
                get(|| async { axum::Json(serde_json::json!({ "n": 7 })) }),
            );
        let client = TestClient::new(app);

        client
            .get("/")
            .await
            .assert_ok()
            .assert_see("hello")
            .assert_dont_see("goodbye");
        client
            .get("/api/n")
            .await
            .assert_ok()
            .assert_json_has("/n")
            .assert_json_at("/n", &serde_json::json!(7));
    }
}
