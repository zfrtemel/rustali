//! Build an OpenAPI 3.1 document from the route registry — a pure function, no server.

use rustali_core::router::{RouteDef, RouteTable};
use serde_json::{Map, Value, json};

use crate::config::OpenApiConfig;

/// Build the OpenAPI document for every route in `table`.
///
/// One `paths` entry per distinct path, one operation per method. Path parameters are read
/// from the `{...}` segments of the path (typed `string`, since the registry records no type
/// info). `operationId` is the route name when set, else the handler; the tag is the
/// controller part of the handler (`PostController::index` → `PostController`); a note is
/// added to operations guarded by `#[authorize]`.
#[must_use]
pub fn build_spec(table: &RouteTable, config: &OpenApiConfig) -> Value {
    let mut paths: Map<String, Value> = Map::new();

    for route in table.routes() {
        let operation = build_operation(route);
        let method = route.method.to_lowercase();
        paths
            .entry(route.path.to_owned())
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .map(|path_item| path_item.insert(method, operation));
    }

    json!({
        "openapi": "3.1.0",
        "info": {
            "title": config.title,
            "version": config.version,
        },
        "paths": Value::Object(paths),
    })
}

/// Build one OpenAPI operation object from a route.
fn build_operation(route: &RouteDef) -> Value {
    let mut operation = Map::new();

    operation.insert(
        "operationId".to_owned(),
        json!(route.name.unwrap_or(route.handler)),
    );
    operation.insert("tags".to_owned(), json!([tag_of(route.handler)]));

    let params = path_parameters(route.path);
    if !params.is_empty() {
        operation.insert("parameters".to_owned(), Value::Array(params));
    }

    // The registry carries no response schema; advertise a generic success.
    operation.insert(
        "responses".to_owned(),
        json!({ "200": { "description": "Successful response" } }),
    );

    if let Some(ability) = route.authorize {
        operation.insert(
            "description".to_owned(),
            json!(format!("Requires the `{ability}` ability.")),
        );
    }

    Value::Object(operation)
}

/// The OpenAPI `parameters` array for the `{name}` segments of `path`.
fn path_parameters(path: &str) -> Vec<Value> {
    param_names(path)
        .into_iter()
        .map(|name| {
            json!({
                "name": name,
                "in": "path",
                "required": true,
                "schema": { "type": "string" },
            })
        })
        .collect()
}

/// Extract `{name}` segment names from an axum/OpenAPI path.
fn param_names(path: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut rest = path;
    while let Some(open) = rest.find('{') {
        rest = &rest[open + 1..];
        if let Some(close) = rest.find('}') {
            let name = &rest[..close];
            if !name.is_empty() {
                names.push(name);
            }
            rest = &rest[close + 1..];
        } else {
            break;
        }
    }
    names
}

/// The tag for an operation: the controller part of a `Controller::method` handler.
fn tag_of(handler: &str) -> &str {
    handler.split("::").next().unwrap_or(handler)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn route(method: &'static str, path: &'static str, handler: &'static str) -> RouteDef {
        RouteDef {
            method,
            path,
            name: None,
            handler,
            middleware: &[],
            authorize: None,
        }
    }

    #[test]
    fn param_names_reads_braces() {
        assert_eq!(param_names("/posts/{id}"), vec!["id"]);
        assert_eq!(param_names("/a/{x}/b/{y}"), vec!["x", "y"]);
        assert_eq!(param_names("/posts"), Vec::<&str>::new());
    }

    #[test]
    fn tag_is_the_controller() {
        assert_eq!(tag_of("PostController::index"), "PostController");
        assert_eq!(tag_of("bare"), "bare");
    }

    #[test]
    fn spec_has_paths_methods_and_params() {
        let mut table = RouteTable::default();
        table.extend([
            route("GET", "/posts", "PostController::index"),
            route("GET", "/posts/{id}", "PostController::show"),
        ]);
        let config = OpenApiConfig::default();
        let spec = build_spec(&table, &config);

        assert_eq!(spec["openapi"], "3.1.0");
        assert_eq!(spec["paths"]["/posts"]["get"]["tags"][0], "PostController");
        assert_eq!(
            spec["paths"]["/posts/{id}"]["get"]["operationId"],
            "PostController::show"
        );
        let params = &spec["paths"]["/posts/{id}"]["get"]["parameters"];
        assert_eq!(params[0]["name"], "id");
        assert_eq!(params[0]["in"], "path");
    }

    #[test]
    fn name_becomes_operation_id_and_authorize_noted() {
        let mut r = route("POST", "/posts", "PostController::store");
        r.name = Some("posts.store");
        r.authorize = Some("post.create");
        let mut table = RouteTable::default();
        table.extend([r]);
        let spec = build_spec(&table, &OpenApiConfig::default());

        let op = &spec["paths"]["/posts"]["post"];
        assert_eq!(op["operationId"], "posts.store");
        assert!(op["description"].as_str().unwrap().contains("post.create"));
    }
}
