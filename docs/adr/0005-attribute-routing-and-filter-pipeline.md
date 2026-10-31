# ADR-0005: Attribute routing on handlers; filters as tower layers

- **Status:** Accepted
- **Date:** 2026-11-01

## Context

VISION §4.5 names two of the five differentiators: .NET-style attribute routing (the
route lives on the handler, no separate route file) and a layered filter pipeline
(global → controller → action middleware, authorization filters, exception filter).
axum provides an explicit `Router` and tower layers but no declarative front-end.

## Decision

1. **`#[controller("/prefix")]` is an attribute macro on an inherent `impl`.** It
   consumes `#[get|post|put|patch|delete|head|options("/path", name = "...")]`,
   `#[middleware(expr)]` and `#[authorize(...)]` on the methods, strips them, and emits
   an implementation of `rustali_core::router::Controller` (`router()` and
   `routes()`), plus an `inventory` registration. Handlers stay ordinary axum
   handlers: `async fn` without `self`, using extractors. The type is a stateless
   namespace.
2. **Composition is pure tower.** Method-level `#[middleware]` and `#[authorize]`
   wrap the handler with `axum::handler::Handler::layer`; controller-level
   `#[middleware]` is `Router::layer` on the controller's router; global layers go
   through `AppBuilder::layer`. Several methods on one path merge into one
   `MethodRouter`.
3. **Mounting is explicit but cheap.** `App::builder().controller::<C>()` mounts one;
   `.controllers()` mounts everything linked in (via `inventory`). Nothing is mounted
   implicitly.
4. **Route metadata is first-class.** Every route produces a `RouteDef` (method,
   path, name, handler, middleware strings, ability). They are collected into a
   `RouteTable` service; the built-in `routes` console command renders it. Plain axum
   routers added with `.routes()` are not listed unless described with `.describe()`.
5. **Authorization = `Gate` + `Policy` + `ActorResolver`.** `#[authorize("ability")]`
   emits an `AuthorizeLayer`; at request time it resolves the actor through the
   registered `ActorResolver` (provided later by the auth package), evaluates the
   ability in the `Gate` (`Policy` implementations, plus built-ins `authenticated`
   and `role:<name>`) and answers 401 (anonymous) or 403 (denied). Layers reach the
   application through an `AppContext` stored in request extensions by the default
   stack.
6. **The exception filter is overridable.** `AppBuilder::on_error(renderer)` is
   consulted before the built-in JSON/HTML rendering; `Retry-After` and
   `WWW-Authenticate` survive re-rendering.
7. **`Throttle`** ships as a fixed-window, in-memory layer with `Retry-After` and
   `X-RateLimit-Remaining` (nested limits report the most restrictive number). A
   shared store comes with the cache package.
8. **No separate `rustali-router` crate.** The registry is ~150 lines; it lives in
   `rustali_core::router`. The plan's crate list is amended accordingly.

## Consequences

- A controller reads like .NET/Spring: prefix on the impl, verbs on the methods,
  filters as attributes — while `cargo expand` shows plain axum code.
- `routes` gives Laravel's `route:list` back, which attribute routing otherwise loses.
- Named routes exist in metadata; URL generation (`route("posts.show", id)`) arrives
  with the view layer (Faz 4), where it is needed.
- Resource controllers (`#[resource]`) and `any` routes are deferred until there is
  a concrete need.
- Misuse is rejected at compile time with targeted messages (`trybuild` tests):
  `self` receivers, non-`async` handlers, `#[middleware]` without a route, an impl with
  no routes, unknown options.
