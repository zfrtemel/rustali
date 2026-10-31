# ADR-0002: One error type, rendered by a negotiation middleware

- **Status:** Accepted
- **Date:** 2026-10-31

## Context

Rustali serves server-rendered HTML and JSON APIs from the same application
(VISION §1). A failing handler must produce a 404 page for a browser and an RFC 9457
*Problem Details* document for a `fetch()` call, without the handler knowing which one
it is talking to. axum's `IntoResponse` has no access to the request, so the error
alone cannot decide.

## Decision

1. A single `rustali_core::Error` enum is the framework-wide error type. Variants carry
   HTTP meaning (`NotFound`, `BadRequest`, `Unauthorized`, `Forbidden`, `Config`,
   `Internal`). Handlers return `Result<T>` and use `?`.
2. `Error::into_response` produces a plain-text response **and attaches an
   `ErrorBody` extension** (status, title, public detail, debug detail).
3. The default middleware stack includes `negotiate_errors`, an axum
   `from_fn_with_state` layer. It inspects the request (`Accept`, path) *before* calling
   the inner service and, if the response carries `ErrorBody`, re-renders it:
   - JSON (`application/problem+json`) when `Accept` mentions JSON, or when it does not
     ask for HTML and the path starts with `/api/`;
   - otherwise an HTML page.
4. `detail` is only populated from `Error::public_detail()` (currently `BadRequest`),
   or from the full error text when `app.debug = true`. Production never leaks internals.
5. Unmatched routes go through a `fallback` that returns `Error::NotFound`, so 404s
   look exactly like every other error. Panics are converted to 500 by
   `CatchPanicLayer`, outside the negotiation layer.

## Consequences

- Handlers stay oblivious to representation; the same code serves SSR and API.
- Applications that need custom pages will override rendering through an
  `ErrorHandler` hook (Faz 4, when the view crate exists) that receives `ErrorBody`.
- Large error payloads must be boxed (`Config(Box<figment::Error>)`) to keep `Result`
  small; clippy enforces this (`result_large_err`).
- Future variants (`Validation`, `Database`) extend the enum; `#[non_exhaustive]`
  keeps that from being a breaking change for downstream matches.

## Alternatives considered

- *Return different error types per route style* — rejected: duplicates logic and
  breaks "one controller, two modes".
- *Decide format inside `IntoResponse` using a task-local* — rejected: hidden state,
  hard to test, surprising for users.
