//! Serve the embedded Swagger UI — offline, no CDN.
//!
//! The JS bundle and CSS from the pinned Swagger UI distribution are compiled into the binary
//! with `include_bytes!`, and the page is a small template that loads them and points at the
//! spec endpoint.

use axum::{
    http::header,
    response::{Html, IntoResponse, Response},
};

/// The Swagger UI `JavaScript` bundle (vendored).
const SWAGGER_UI_JS: &[u8] = include_bytes!("../assets/swagger-ui-bundle.js");
/// The Swagger UI stylesheet (vendored).
const SWAGGER_UI_CSS: &[u8] = include_bytes!("../assets/swagger-ui.css");

/// The HTML page that boots Swagger UI against `spec_path`, loading assets relative to
/// `ui_path`.
pub(crate) fn index_html(ui_path: &str, spec_path: &str, title: &str) -> Html<String> {
    let ui = ui_path.trim_end_matches('/');
    Html(format!(
        r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <link rel="stylesheet" href="{ui}/swagger-ui.css">
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="{ui}/swagger-ui-bundle.js"></script>
  <script>
    window.ui = SwaggerUIBundle({{
      url: "{spec_path}",
      dom_id: "#swagger-ui",
      deepLinking: true,
    }});
  </script>
</body>
</html>"##
    ))
}

/// Serve one of the vendored asset files by name, or `404` for anything else.
#[must_use]
pub(crate) fn asset(file: &str) -> Response {
    match file {
        "swagger-ui-bundle.js" => (
            [(header::CONTENT_TYPE, "application/javascript")],
            SWAGGER_UI_JS,
        )
            .into_response(),
        "swagger-ui.css" => ([(header::CONTENT_TYPE, "text/css")], SWAGGER_UI_CSS).into_response(),
        _ => axum::http::StatusCode::NOT_FOUND.into_response(),
    }
}
