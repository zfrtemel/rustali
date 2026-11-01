# Authorization

Authorization answers "is this user allowed to do this?" Rustali models it as named
**abilities** checked by a **Gate**, enforced declaratively with `#[authorize("...")]` on a
route.

## Built-in abilities

Two work out of the box against the authenticated user (see
[Authentication](authentication)):

- `authenticated` — passes for any logged-in user.
- `role:<name>` — passes when the user's `roles()` contains `<name>`.

```rust
#[post("/posts")]
#[authorize(role = "editor")]        // sugar for the ability "role:editor"
async fn store(/* ... */) -> Result<Redirect> { /* ... */ }

#[delete("/posts/{id}")]
#[authorize("authenticated")]
async fn destroy(/* ... */) -> Result<StatusCode> { /* ... */ }
```

At request time the filter resolves the current user, evaluates the ability, and answers
**401** when nobody is logged in or **403** when the user is present but denied — before the
handler runs.

## Custom policies

For anything beyond roles, implement `Policy`: give it an ability name and an async
decision. It receives a `Check` with the context, the (optional) actor, and the request
parts.

```rust
use rustali::prelude::*;

pub struct UpdatePost;

impl Policy for UpdatePost {
    fn ability(&self) -> &'static str { "post.update" }

    fn allows<'a>(&'a self, check: Check<'a>) -> AuthFuture<'a, bool> {
        Box::pin(async move {
            let Some(actor) = check.actor else { return Ok(false) };
            // e.g. only the owner, whose id is in the path, may update
            Ok(check.parts.uri.path().ends_with(&format!("/{}", actor.id())))
        })
    }
}
```

`Check` gives you:

| Field | Is |
|---|---|
| `check.ctx` | the `AppContext` (database, services, config) |
| `check.actor` | `Option<&dyn Actor>` — the current user (`id()`, `roles()`) |
| `check.parts` | the request head (path, headers, extensions) |

Register policies on the builder, then reference the ability from a route:

```rust
App::builder()
    .policy(UpdatePost)
    // ...
    .controllers()
```

```rust
#[patch("/posts/{id}")]
#[authorize("post.update")]
async fn update(/* ... */) -> Result<Json<Post>> { /* ... */ }
```

Needing the concrete user type inside a policy? Downcast the actor:

```rust
let user = check.actor.and_then(|a| a.as_any().downcast_ref::<User>());
```

## Checking by hand

You are not limited to the route filter. Evaluate an ability anywhere you have the gate —
though in a handler the `#[authorize]` attribute is almost always cleaner.

## Where authorization sits

`#[authorize]` emits a tower layer that runs inside the default middleware stack, so it
sees the injected `AppContext` and the resolved user. Because it is a layer, the decision
happens before your handler body — a denied request never touches your code. An unknown
ability is denied and logged, so a typo fails closed, not open.

> `rustali make:policy <Name>` scaffolds a `Policy` for you.
