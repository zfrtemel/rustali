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

Configure them in `.env`:

```bash
# .env
SESSION_COOKIE=rustali_session
SESSION_LIFETIME_MINUTES=120     # inactivity expiry
SESSION_SAME_SITE=lax            # lax | strict | none
SESSION_CSRF=true                # CSRF protection on/off
```

Cookies are `HttpOnly`, `SameSite` as configured, signed with `app.key`, and `Secure` in
production. The store is in-memory for now (fine for a single node); a shared store is
future work.

### The cookie, field by field

| Setting | Value | Why |
|---|---|---|
| `HttpOnly` | always | JavaScript cannot read the cookie, so an XSS bug cannot steal the session. |
| `SameSite` | `same_site` config | `lax` (the default) blocks the cookie on most cross-site requests; `strict` is stricter; `none` allows cross-site (and forces `Secure`). |
| signed | with `app.key` | the client cannot tamper with the cookie without detection. |
| `Secure` | in production | the cookie is only sent over HTTPS outside development. |

Because the store is per-process and in-memory, sessions do not survive a restart and are not
shared across instances yet — the same caveat as the memory [cache](../deeper/cache).

## Using the session

The raw `Session` extractor reads and writes arbitrary keyed values. Values round-trip
through JSON, so any `Serialize + DeserializeOwned` type works — `get` returns `None` for a
missing key:

```rust
#[get("/visit")]
async fn visit(session: Session) -> Result<String> {
    let count: i64 = session.get("count").await?.unwrap_or(0);
    session.insert("count", count + 1).await?;
    Ok(format!("visit #{}", count + 1))
}
```

The flash, old-input and CSRF helpers below are all built on this same session — they are
conveniences for patterns you would otherwise hand-roll with `get`/`insert`.

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
etc.). Each queues one `Message` carrying a `level` and `text`.

The full post-redirect-get loop: the action queues a message and redirects; the next page
`take()`s the queued messages (which also clears them, so a refresh does not show them again)
and the template renders them.

```rust
// 1. the action queues a message, then redirects
#[post("/posts")]
async fn store(db: Db, flash: Flash, /* ... */) -> Result<Redirect> {
    // ... create the post ...
    flash.success("Post published.").await;
    Ok(Redirect::to("/"))
}

// 2. the page it lands on drains the queue into its view
#[get("/")]
async fn index(flash: Flash) -> Result<View<Home>> {
    Ok(View::new(Home {
        flash: flash.take().await,        // Vec<Message>, cleared after this call
        // ...
    }))
}
```

```rust
// on the view struct
struct Home {
    flash: Vec<rustali::session::Message>,
    // ...
}
```

```html
{% for message in flash %}
  <div class="flash {{ message.level }}">{{ message.text }}</div>
{% endfor %}
```

<code v-pre>{{ message.level }}</code> renders as `success` / `info` / `warning` / `error`
(the `Level` enum implements `Display`), so it doubles as a CSS class.

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

### How the check works

The token is a random value stored in the visitor's session the first time it is needed. On
an unsafe request the layer compares the submitted token (from the `_token` field or the
`X-CSRF-Token` header) against the one in the session; a match passes, anything else is a
403. Because the token lives in the session — tied to the signed cookie — an attacker on
another site cannot read it to forge a request, which is the whole point of CSRF protection.

### From a single-page app or fetch()

When the page is driven by JavaScript rather than a server-rendered `<form>`, read the token
once and send it as a header on every mutating request:

```rust
// expose the token to the page (e.g. in a <meta> tag or a JSON bootstrap)
#[get("/app")]
async fn app_shell(csrf: Csrf) -> Result<View<AppShell>> {
    Ok(View::new(AppShell { csrf_token: csrf.token().await }))
}
```

```js
// then on the client
fetch("/posts", {
  method: "POST",
  headers: { "Content-Type": "application/json", "X-CSRF-Token": token },
  body: JSON.stringify({ title, body }),
});
```

A pure API that authenticates with bearer tokens does not need any of this: `/api/*` paths
are exempt, because a bearer token is not sent automatically by the browser the way a cookie
is, so there is nothing to forge.

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
