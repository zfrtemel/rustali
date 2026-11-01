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

Two syntaxes, one mechanism:

- `#[authorize("ability")]` names an ability directly — `authenticated`, `role:editor`, or
  any custom policy's ability like `post.update`.
- `#[authorize(role = "editor")]` is sugar that expands to the ability `role:editor`.

At request time the filter resolves the current user, evaluates the ability, and answers
**401** when nobody is logged in or **403** when the user is present but denied — before the
handler runs. The distinction matters to clients: 401 means "log in", 403 means "you are
logged in but not allowed".

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

### A realistic ownership policy

Most custom policies load the subject and compare it to the actor. Here, only a post's
author may edit it — the policy reads the id from the path, loads the post, and checks
ownership against the database:

```rust
pub struct UpdatePost;

impl Policy for UpdatePost {
    fn ability(&self) -> &'static str { "post.update" }

    fn allows<'a>(&'a self, check: Check<'a>) -> AuthFuture<'a, bool> {
        Box::pin(async move {
            // must be logged in
            let Some(actor) = check.actor else { return Ok(false) };

            // id from the matched path, e.g. /posts/{id}
            let Some(id) = check.parts.uri.path().rsplit('/').next().and_then(|s| s.parse::<i64>().ok())
            else {
                return Ok(false);
            };

            // load the subject and compare owners
            let db = check.ctx.db()?;
            let Some(post) = Post::find(&*db, id).await? else { return Ok(false) };
            Ok(post.user_id.to_string() == actor.id())
        })
    }
}
```

A policy returns `Ok(false)` to deny and `Err(..)` only for a real failure (a database error,
say). Denying by returning `false` produces a clean 403; an `Err` propagates as a 500,
because it means something broke, not that the user lacked permission.

## Checking inside a handler

`#[authorize]` is the cleanest way to guard a whole route, but sometimes the decision depends
on data you only have mid-handler, or you want to branch rather than reject. Then take the
user as an extractor and decide in plain Rust:

```rust
#[get("/posts/{id}/edit")]
async fn edit(Auth(user): Auth<User>, db: Db, Path(id): Path<i64>) -> Result<View<EditPost>> {
    let post = Post::find_or_404(&db, id).await?;
    if post.user_id != user.id {
        return Err(Error::Forbidden);   // 403, negotiated to JSON or an error page
    }
    Ok(View::new(EditPost { post }))
}
```

Returning `Error::Forbidden` / `Error::Unauthorized` gives the same 403 / 401 the filter
would, so hand-rolled checks and `#[authorize]` stay consistent. Reach for the attribute when
the rule is "may this user reach this route at all"; reach for an in-handler check when the
rule depends on the loaded record or you want to show a different result rather than deny.

## Applying a policy without the attribute

`#[authorize("ability")]` expands to a tower layer, and you can add that same layer to a
router by hand with `AuthorizeLayer::new("ability")` — useful to guard a whole group of
routes at once:

```rust
use rustali::auth::AuthorizeLayer;

let admin = admin_routes().layer(AuthorizeLayer::new("role:admin"));
```

It needs the default middleware stack (which injects the context and resolves the user), so
it works on routers built through `App::builder()`.

## Where authorization sits

`#[authorize]` emits a tower layer that runs inside the default middleware stack, so it
sees the injected `AppContext` and the resolved user. Because it is a layer, the decision
happens before your handler body — a denied request never touches your code. An unknown
ability is denied and logged, so a typo fails closed, not open.

> `rustali make:policy <Name>` scaffolds a `Policy` for you.
