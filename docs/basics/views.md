# Views

Views render HTML. By default Rustali uses **Askama**, which compiles your templates into
Rust and checks them at build time — a missing variable or a typo in a template is a
compile error, not a 500 in production. If you want runtime templates with hot reload,
the **Tera** backend is available behind a feature.

## Askama (default)

A template is a struct that derives `Template` and names a template file. Its fields are
the variables available in the template.

```rust
use rustali::prelude::*;   // brings `View` and `Template` into scope

#[derive(Template)]
#[template(path = "posts/index.html")]
struct PostsIndex {
    posts: Vec<Post>,
    app_name: String,
}

#[get("/")]
async fn index(ctx: Ctx, db: Db) -> Result<View<PostsIndex>> {
    Ok(View::new(PostsIndex {
        posts: Post::query().order_by_desc(Post::ID).fetch_all(&db).await?,
        app_name: ctx.config().app.name.clone(),
    }))
}
```

Templates live under `templates/` by default. Askama reads an `askama.toml` from the
crate root:

```toml
# askama.toml
[general]
dirs = ["templates"]
```

```html
<!-- templates/posts/index.html -->
<h1>{{ app_name }}</h1>
{% for post in posts %}
  <article><h2>{{ post.title }}</h2><p>{{ post.body }}</p></article>
{% endfor %}
```

`View::new(page)` returns a `200 text/html` response. Set a different status with
`with_status` — handy for re-rendering an invalid form:

```rust
Ok(View::new(login_page).with_status(StatusCode::UNPROCESSABLE_ENTITY))
```

Because `View<T>` is just an `IntoResponse`, a handler that returns `Result<View<T>>` gets
automatic error rendering like any other.

## Layouts

Askama supports template inheritance. A base layout declares blocks; pages extend it and
fill them in.

```html
<!-- templates/layout.html -->
<!doctype html>
<html>
<head><title>{% block title %}{{ app_name }}{% endblock %}</title></head>
<body><main>{% block content %}{% endblock %}</main></body>
</html>
```

```html
<!-- templates/posts/index.html -->
{% extends "layout.html" %}
{% block title %}Posts{% endblock %}
{% block content %}
  {% for post in posts %}<article>{{ post.title }}</article>{% endfor %}
{% endblock %}
```

## Escaping

Askama auto-escapes HTML in `.html` templates. To emit a trusted, pre-rendered string
(like a CSRF hidden input), pipe it through `safe`:

```html
<form method="post" action="/posts">
  {{ csrf_field|safe }}
  ...
</form>
```

## Forms: errors and old input

When a [validated](validation) form submission fails, the errors and submitted values are
flashed to the session and the request redirects back. Pull them into the template struct
when you render the form:

```rust
#[get("/")]
async fn index(ctx: Ctx, db: Db, flash: Flash, csrf: Csrf) -> Result<View<PostsIndex>> {
    let errors = flash.errors().await;
    let page = PostsIndex {
        app_name: ctx.config().app.name.clone(),
        posts: Post::query().order_by_desc(Post::ID).fetch_all(&db).await?,
        csrf_field: csrf.field().await,
        old_title: flash.old("title").await.unwrap_or_default(),
        title_error: errors.first("title").map(str::to_owned),
    };
    flash.clear_old().await;
    Ok(View::new(page))
}
```

```html
<input name="title" value="{{ old_title }}">
{% if let Some(message) = title_error %}<p class="error">{{ message }}</p>{% endif %}
```

See [CSRF & Sessions](../security/csrf) for `Flash` and `Csrf` in full.

## Flash messages

`flash.take()` returns and clears the queued messages, which you render in your layout:

```rust
flash: Vec<rustali::session::Message>,   // on the template struct
// flash: flash.take().await,            // when building it
```

```html
{% for message in flash %}
  <div class="flash {{ message.level }}">{{ message.text }}</div>
{% endfor %}
```

A `Message` has a `level` (`success`, `info`, `warning`, `error`) and `text`.

## Tera (runtime, `tera` feature)

Enable the `tera` feature for runtime templates. Register the engine with `TeraProvider`,
then render by name with a serialisable context:

```rust
async fn index(views: Inject<TeraView>) -> Result<Html<String>> {
    views.render("posts/index.html", &serde_json::json!({ "posts": posts }))
}
```

Tera reads `views/**/*` by default (configurable in `config/view.toml`). Use Askama when
you want compile-time safety; reach for Tera when designers need to edit templates without
a recompile.

## SSR and JSON together

A controller can serve an HTML `View` on one route and `Json` on another — the same
handler style, different return type. Errors are rendered to match the caller (HTML page
for browsers, problem+JSON for API clients), so one controller cleanly serves both. See
[Responses & Errors](responses).
