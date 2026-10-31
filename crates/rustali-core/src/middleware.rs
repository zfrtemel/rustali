//! Built-in middleware and the default stack applied to every application.
//!
//! Order (outermost first): request id → tracing → panic catching → context
//! injection → error negotiation → user layers → routes. Everything is a plain tower
//! layer, so applications can add their own with [`crate::AppBuilder::layer`].

pub mod throttle;

use std::sync::Arc;

use axum::{
    Router,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{Next, from_fn_with_state},
    response::{Html, IntoResponse, Response},
};
pub use tower_http::cors::{Any as CorsAny, CorsLayer as Cors};
use tower_http::{
    catch_panic::CatchPanicLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

pub use self::throttle::Throttle;
use crate::{AppContext, Error, error::ErrorBody};

/// Header carrying the per-request identifier.
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Media type of RFC 9457 problem documents.
pub const PROBLEM_JSON: &str = "application/problem+json";

/// Headers that an error response keeps when it is re-rendered.
const PRESERVED_HEADERS: &[header::HeaderName] = &[header::RETRY_AFTER, header::WWW_AUTHENTICATE];

/// Wrap `router` in the default stack. Called by the app builder.
pub(crate) fn apply_defaults(router: Router<AppContext>, ctx: AppContext) -> Router<AppContext> {
    router
        .fallback(not_found)
        .layer(from_fn_with_state(ctx.clone(), negotiate_errors))
        .layer(from_fn_with_state(ctx, inject_context))
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
}

/// Fallback for unmatched routes, so 404s render like every other error.
async fn not_found() -> Error {
    Error::NotFound
}

/// Store the [`AppContext`] in request extensions so plain tower layers (authorize,
/// throttle, user middleware) can reach services without axum state.
pub async fn inject_context(
    State(ctx): State<AppContext>,
    mut request: Request,
    next: Next,
) -> Response {
    request.extensions_mut().insert(ctx);
    next.run(request).await
}

/// Facts about the failed request handed to a custom error renderer.
#[derive(Debug, Clone)]
pub struct ErrorRequest {
    /// Request path.
    pub path: String,
    /// `true` when the client prefers JSON.
    pub wants_json: bool,
    /// Request id, when the default stack assigned one.
    pub request_id: Option<String>,
    /// Debug mode is on, so detail may be shown.
    pub debug: bool,
}

/// Application-provided error renderer (the "exception filter" override).
///
/// Return `Some(response)` to take over, `None` to fall back to the built-in
/// JSON/HTML rendering. Installed with [`crate::AppBuilder::on_error`].
pub type ErrorRenderer =
    Arc<dyn Fn(&ErrorBody, &ErrorRequest) -> Option<Response> + Send + Sync + 'static>;

/// Service wrapper for the installed [`ErrorRenderer`].
#[derive(Clone)]
pub struct ErrorRendererHandle(pub ErrorRenderer);

/// Render [`ErrorBody`] responses as JSON or HTML depending on the client.
///
/// JSON is chosen when the `Accept` header mentions JSON, or when it does not ask
/// for HTML and the path is under `/api/`. Everything else gets an HTML page. A
/// renderer installed with `on_error` is consulted first.
pub async fn negotiate_errors(
    State(ctx): State<AppContext>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_owned();
    let wants_json = wants_json(request.headers(), &path);
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    let response = next.run(request).await;
    let Some(body) = response.extensions().get::<ErrorBody>().cloned() else {
        return response;
    };

    let debug = ctx.config().app.debug;
    let info = ErrorRequest {
        path,
        wants_json,
        request_id,
        debug,
    };

    let custom = ctx
        .services()
        .get::<ErrorRendererHandle>()
        .and_then(|renderer| (renderer.0)(&body, &info));

    let mut rendered = custom.unwrap_or_else(|| {
        let detail = body
            .detail
            .clone()
            .or_else(|| debug.then(|| body.debug_detail.clone()));
        if wants_json {
            problem_json(&body, detail, &info.path, info.request_id.as_deref())
        } else {
            error_page(&body, detail)
        }
    });

    for name in PRESERVED_HEADERS {
        if let Some(value) = response.headers().get(name) {
            rendered.headers_mut().insert(name.clone(), value.clone());
        }
    }
    rendered
}

/// Decide whether a client prefers a JSON representation.
#[must_use]
pub fn wants_json(headers: &HeaderMap, path: &str) -> bool {
    let accept = headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if accept.contains("text/html") {
        return false;
    }
    accept.contains("json") || path.starts_with("/api/")
}

fn problem_json(
    body: &ErrorBody,
    detail: Option<String>,
    instance: &str,
    request_id: Option<&str>,
) -> Response {
    let mut doc = serde_json::json!({
        "type": "about:blank",
        "title": body.title,
        "status": body.status.as_u16(),
        "instance": instance,
    });
    if let Some(detail) = detail {
        doc["detail"] = detail.into();
    }
    if let Some(id) = request_id {
        doc["request_id"] = id.into();
    }
    if let Some(errors) = &body.validation {
        doc["errors"] = serde_json::to_value(&errors.fields).unwrap_or_default();
    }
    (
        body.status,
        [(header::CONTENT_TYPE, HeaderValue::from_static(PROBLEM_JSON))],
        doc.to_string(),
    )
        .into_response()
}

fn error_page(body: &ErrorBody, detail: Option<String>) -> Response {
    let detail_html = detail
        .map(|d| format!("<p class=\"detail\">{}</p>", escape(&d)))
        .unwrap_or_default();
    let html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<title>{code} {title}</title>\
<style>body{{font-family:system-ui,sans-serif;display:grid;place-items:center;min-height:100vh;margin:0;background:#fafafa;color:#222}}\
main{{text-align:center;padding:2rem}}h1{{font-size:5rem;margin:0;font-weight:700;color:#b7410e}}\
h2{{font-weight:400;margin:.5rem 0 1rem}}.detail{{font-family:ui-monospace,monospace;background:#fff;border:1px solid #ddd;padding:1rem;text-align:left;white-space:pre-wrap}}</style>\
</head><body><main><h1>{code}</h1><h2>{title}</h2>{detail_html}</main></body></html>",
        code = body.status.as_u16(),
        title = escape(body.title),
    );
    (body.status, Html(html)).into_response()
}

/// Minimal HTML escaping for error pages.
#[must_use]
pub fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// Status used by [`CatchPanicLayer`] when a handler panics.
pub const PANIC_STATUS: StatusCode = StatusCode::INTERNAL_SERVER_ERROR;

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(accept: Option<&str>) -> HeaderMap {
        let mut map = HeaderMap::new();
        if let Some(value) = accept {
            map.insert(
                header::ACCEPT,
                HeaderValue::from_str(value).unwrap_or_else(|_| HeaderValue::from_static("*/*")),
            );
        }
        map
    }

    #[test]
    fn browsers_get_html_even_under_api() {
        let h = headers(Some("text/html,application/xhtml+xml,*/*;q=0.8"));
        assert!(!wants_json(&h, "/api/users"));
    }

    #[test]
    fn json_clients_get_json() {
        assert!(wants_json(&headers(Some("application/json")), "/users"));
        assert!(wants_json(
            &headers(Some("application/problem+json")),
            "/users"
        ));
    }

    #[test]
    fn api_prefix_defaults_to_json() {
        assert!(wants_json(&headers(None), "/api/users"));
        assert!(wants_json(&headers(Some("*/*")), "/api/users"));
        assert!(!wants_json(&headers(Some("*/*")), "/users"));
    }

    #[test]
    fn html_is_escaped() {
        assert_eq!(
            escape("<b>\"x\" & 'y'</b>"),
            "&lt;b&gt;&quot;x&quot; &amp; &#39;y&#39;&lt;/b&gt;"
        );
    }
}
