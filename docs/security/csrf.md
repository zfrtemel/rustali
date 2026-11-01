# CSRF & Sessions

Sessions store per-visitor data in a signed cookie. On top of them Rustali provides flash
messages, old-input for forms, and CSRF protection — the pieces a server-rendered app
needs. They come from the `sessions` feature (default on) and the `SessionProvider`.

## Enabling sessions

Sessions need a layer that runs with configuration available, so add both the provider and
the layers:

```rust
App::builder()
    .provider(SessionProvider)
    .configure(session_layers)    // cookie session + CSRF layers
    .controllers()
    .run()
    .await
```

Configure them in `config/session.toml`:

```toml
cookie = "rustali_session"
lifetime_minutes = 120     # inactivity expiry
same_site = "lax"          # lax | strict | none
csrf = true                # CSRF protection on/off
```

Cookies are `HttpOnly`, `SameSite` as configured, signed with `app.key`, and `Secure` in
production. The store is in-memory for now (fine for a single node); a shared store is
future work.

## Using the session

The raw `Session` extractor reads and writes arbitrary keyed values:

```rust
#[get("/visit")]
async fn visit(session: Session) -> Result<String> {
    let count: i64 = session.get("count").await?.unwrap_or(0);
    session.insert("count", count + 1).await?;
    Ok(format!("visit #{}", count + 1))
}
```

## Flash messages

`Flash` queues one-request messages that survive exactly one redirect — the
post-redirect-get pattern.

```rust
#[post("/posts")]
async fn store(db: Db, flash: Flash, /* ... */) -> Result<Redirect> {
    // ... create the post ...
    flash.success("Post published.").await;
    Ok(Redirect::to("/"))
}
```

Levels: `success`, `info`, `warning`, `error` (`flash.success(..)`, `flash.error(..)`,
etc.). Render and clear them with `take()`:

```rust
flash: Vec<rustali::session::Message>,   // on the template struct
// flash: flash.take().await,            // when building it
```

```html
{% for message in flash %}
  <div class="flash {{ message.level }}">{{ message.text }}</div>
{% endfor %}
```

A `Message` has `level` and `text`.

## Old input and errors

When a [validated](../basics/validation) form fails, the extractor flashes the errors and
the submitted values and redirects back. Read them when rendering the form again:

```rust
let errors = flash.errors().await;             // ValidationErrors
let old_title = flash.old("title").await;      // Option<String>
errors.first("title");                          // Option<&str>
flash.clear_old().await;                        // after rendering once
```

## CSRF protection

With `csrf = true`, every **unsafe** request (`POST`, `PUT`, `PATCH`, `DELETE`) must carry a
valid token. Safe methods (`GET`, `HEAD`, `OPTIONS`, `TRACE`) and `/api/*` paths — which
authenticate with bearer tokens, not cookies — are exempt. A missing or wrong token is
rejected with `403`.

Embed the token in a form with `Csrf`:

```rust
#[get("/posts/new")]
async fn new_form(csrf: Csrf) -> Result<View<NewPost>> {
    Ok(View::new(NewPost { csrf_field: csrf.field().await }))
}
```

```html
<form method="post" action="/posts">
  {{ csrf_field|safe }}
  <!-- ... -->
</form>
```

`csrf.field()` renders a hidden `<input name="_token" ...>`. For JavaScript requests, send
the raw `csrf.token().await` value in the `X-CSRF-Token` header instead.

Because your form DTO deserialises the whole body, accept the token field so it does not
break deserialisation:

```rust
#[derive(Deserialize, Validate)]
struct CreatePost {
    #[rule(required, length(min = 3, max = 200))]
    title: String,
    #[serde(default, rename = "_token")]
    _token: String,
}
```

> The blog and auth-app examples use all of this together — flash, old input, CSRF — and an
> integration test drives the full flow, including the 403 for a missing token.
