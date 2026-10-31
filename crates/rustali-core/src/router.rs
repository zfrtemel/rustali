//! Controller registry and route metadata behind `#[controller]`.
//!
//! The attribute macro turns an `impl` block into a [`Controller`]: a function that
//! builds an axum router plus the list of [`RouteDef`]s it contains. Controllers are
//! also submitted to an [`inventory`] collection, so an application can pick them up
//! with `App::builder().controllers()` without listing each one — or stay explicit with
//! `.controller::<PostController>()`. The `routes` console command prints the table.

use std::fmt::Write as _;

use axum::Router;

use crate::{AppContext, Command, CommandFuture};

/// Static description of one route, used by `rustali routes` and tooling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDef {
    /// HTTP method, upper-case.
    pub method: &'static str,
    /// Full path including the controller prefix, axum syntax (`/posts/{id}`).
    pub path: &'static str,
    /// Optional route name (`posts.show`).
    pub name: Option<&'static str>,
    /// `Controller::method` for humans.
    pub handler: &'static str,
    /// Middleware expressions applied to this route (method-level first, then
    /// controller-level), stringified.
    pub middleware: &'static [&'static str],
    /// Ability required by `#[authorize]`, if any.
    pub authorize: Option<&'static str>,
}

/// Implemented by `#[controller]` on the annotated type.
pub trait Controller {
    /// Type name, for listings.
    const NAME: &'static str;

    /// Router with every route of this controller, prefixed and layered.
    fn router() -> Router<AppContext>;

    /// Metadata for every route, in declaration order.
    fn routes() -> Vec<RouteDef>;
}

/// Entry stored in the global registry by `#[controller]`.
pub struct Registration {
    /// Controller type name.
    pub name: &'static str,
    /// Builds the router.
    pub router: fn() -> Router<AppContext>,
    /// Lists the routes.
    pub routes: fn() -> Vec<RouteDef>,
}

impl Registration {
    /// Build a registration for `C`. Called by generated code.
    #[must_use]
    pub const fn of<C: Controller>() -> Self {
        Self {
            name: C::NAME,
            router: C::router,
            routes: C::routes,
        }
    }
}

inventory::collect!(Registration);

/// Every controller registered through `#[controller]`, in link order.
pub fn registered() -> impl Iterator<Item = &'static Registration> {
    inventory::iter::<Registration>.into_iter()
}

/// All known routes of the application; stored as a service and shown by `routes`.
#[derive(Debug, Default, Clone)]
pub struct RouteTable {
    routes: Vec<RouteDef>,
}

impl RouteTable {
    /// Add routes (keeps declaration order).
    pub fn extend(&mut self, routes: impl IntoIterator<Item = RouteDef>) {
        self.routes.extend(routes);
    }

    /// All routes.
    #[must_use]
    pub fn routes(&self) -> &[RouteDef] {
        &self.routes
    }

    /// Find a route by name.
    #[must_use]
    pub fn by_name(&self, name: &str) -> Option<&RouteDef> {
        self.routes.iter().find(|r| r.name == Some(name))
    }

    /// Render an aligned text table.
    #[must_use]
    pub fn render(&self) -> String {
        if self.routes.is_empty() {
            return "No controller routes registered. Explicit axum routers are not listed.\n"
                .to_owned();
        }
        let col = |f: fn(&RouteDef) -> usize, min: usize| {
            self.routes.iter().map(f).max().unwrap_or(0).max(min)
        };
        let w_method = col(|r| r.method.len(), 6);
        let w_path = col(|r| r.path.len(), 4);
        let w_name = col(|r| r.name.map_or(0, str::len), 4);
        let w_handler = col(|r| r.handler.len(), 7);

        let mut out = String::new();
        // Writing into a `String` cannot fail.
        let _ = writeln!(
            out,
            "{:<w_method$}  {:<w_path$}  {:<w_name$}  {:<w_handler$}  MIDDLEWARE",
            "METHOD", "PATH", "NAME", "HANDLER"
        );
        for route in &self.routes {
            let mut extras: Vec<String> =
                route.middleware.iter().map(|m| (*m).to_owned()).collect();
            if let Some(ability) = route.authorize {
                extras.push(format!("authorize({ability})"));
            }
            let _ = writeln!(
                out,
                "{:<w_method$}  {:<w_path$}  {:<w_name$}  {:<w_handler$}  {}",
                route.method,
                route.path,
                route.name.unwrap_or("-"),
                route.handler,
                if extras.is_empty() {
                    "-".to_owned()
                } else {
                    extras.join(", ")
                }
            );
        }
        out
    }
}

/// `routes` console command.
pub struct RoutesCommand;

impl Command for RoutesCommand {
    fn name(&self) -> &'static str {
        "routes"
    }

    fn about(&self) -> &'static str {
        "List controller routes with their middleware"
    }

    fn run<'a>(&'a self, ctx: &'a AppContext, _args: Vec<String>) -> CommandFuture<'a> {
        Box::pin(async move {
            let table = ctx.service::<RouteTable>()?;
            print!("{}", table.render());
            Ok(())
        })
    }
}

/// Join a controller prefix and a route path into one axum path.
///
/// `join_path("/posts", "/")` → `/posts`; `join_path("/posts", "{id}")` → `/posts/{id}`;
/// `join_path("", "/")` → `/`.
#[must_use]
pub fn join_path(prefix: &str, path: &str) -> String {
    let prefix = prefix.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    let mut full = String::with_capacity(prefix.len() + path.len() + 2);
    if !prefix.starts_with('/') {
        full.push('/');
    }
    full.push_str(prefix);
    if !path.is_empty() {
        if !full.ends_with('/') {
            full.push('/');
        }
        full.push_str(path);
    }
    if full.is_empty() {
        "/".to_owned()
    } else {
        full
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_path_normalises_slashes() {
        assert_eq!(join_path("/posts", "/"), "/posts");
        assert_eq!(join_path("/posts", "{id}"), "/posts/{id}");
        assert_eq!(join_path("/posts/", "/{id}/"), "/posts/{id}/");
        assert_eq!(join_path("", "/"), "/");
        assert_eq!(join_path("", ""), "/");
        assert_eq!(join_path("api", "ping"), "/api/ping");
    }

    #[test]
    fn table_renders_and_finds_names() {
        let mut table = RouteTable::default();
        table.extend([RouteDef {
            method: "GET",
            path: "/posts/{id}",
            name: Some("posts.show"),
            handler: "PostController::show",
            middleware: &["Throttle::per_minute(60)"],
            authorize: Some("post.view"),
        }]);
        let text = table.render();
        assert!(text.contains("GET"));
        assert!(text.contains("/posts/{id}"));
        assert!(text.contains("authorize(post.view)"));
        assert!(table.by_name("posts.show").is_some());
        assert!(table.by_name("nope").is_none());
        assert!(
            RouteTable::default()
                .render()
                .contains("No controller routes")
        );
    }
}
