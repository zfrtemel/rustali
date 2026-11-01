# Requests & Validation

A handler receives the request body through an extractor. For input you want to trust,
use the **validated** extractors: declare the rules on your DTO with `#[derive(Validate)]`,
then ask for `Validated<Json<T>>` or `Validated<Form<T>>`. Invalid input never reaches
your handler.

## Declaring rules

```rust
use rustali::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Validate)]
struct CreatePost {
    #[rule(required, length(min = 3, max = 200))]
    title: String,

    #[rule(required, length(min = 1))]
    body: String,

    #[rule(email)]
    author_email: Option<String>,

    #[rule(range(min = 1, max = 5))]
    rating: u8,

    #[rule(one_of("draft", "published"))]
    status: String,

    #[rule(required, length(min = 8), confirmed)]
    password: String,
    #[serde(default)]
    password_confirmation: String,
}
```

### The rules

| Rule | Checks |
|---|---|
| `required` | Non-empty / non-blank (strings), `Some` (options). |
| `length(min = N, max = N)` | Character count in range (either bound optional). |
| `range(min = N, max = N)` | Numeric value in range. |
| `email` | A valid email shape. |
| `url` | An `http`/`https` URL. |
| `alpha_num` | Letters and digits only. |
| `one_of("a", "b")` | Value is one of the listed options. |
| `confirmed` | Equals the `<field>_confirmation` field. |
| `custom = "path"` | A function `fn(&T) -> Result<(), impl Display>`. |
| `nested` | The field itself implements `Validate`. |

Several rules combine in one attribute: `#[rule(required, length(min = 3, max = 200))]`.
On an `Option<T>` field every rule except `required` is **skipped when the value is
`None`** — so `#[rule(email)] author_email: Option<String>` validates the address only
when one is supplied. All failures across all fields are collected, not just the first.

### Custom rules

```rust
#[derive(Deserialize, Validate)]
struct Comment {
    #[rule(custom = "no_links")]
    body: String,
}

fn no_links(body: &String) -> Result<(), String> {
    if body.contains("http") { Err("Links are not allowed.".into()) } else { Ok(()) }
}
```

### Nested validation

```rust
#[derive(Deserialize, Validate)]
struct Order {
    #[rule(nested)]
    address: Address,   // Address: Validate
}
```

Errors from the child are keyed under the parent, e.g. `address.city`.

## Using it in a handler

### JSON APIs

```rust
#[post("/api/posts")]
async fn store(db: Db, Validated(Json(input)): Validated<Json<CreatePost>>) -> Result<Json<Post>> {
    let post = Post::new(&input.title, &input.body).insert(&db).await?;
    Ok(Json(post))
}
```

On failure the client gets `422 Unprocessable Entity` as an RFC 9457 problem document
with the field errors under `errors`:

```json
{
  "title": "Unprocessable Entity",
  "status": 422,
  "detail": "The given data was invalid.",
  "errors": {
    "title": ["The title must be between 3 and 200 characters."],
    "rating": ["The rating must be between 1 and 5."]
  }
}
```

### HTML forms

The *same* extractor behaves differently for a browser form: on failure it flashes the
errors and the submitted values into the session and **redirects back**, so the form
re-renders filled in with messages. (Without the session package installed, it falls back
to the 422 page.)

```rust
#[post("/posts")]
async fn store(
    db: Db,
    flash: Flash,
    Validated(Form(input)): Validated<Form<CreatePost>>,
) -> Result<Redirect> {
    Post::new(input.title.trim(), input.body.trim()).insert(&db).await?;
    flash.success("Post published.").await;
    Ok(Redirect::to("/"))
}
```

Reading the errors and old input back on the redirected request is covered in
[Views](views) and [CSRF & Sessions](../security/csrf). In short:

```rust
let errors = flash.errors().await;          // ValidationErrors
let old_title = flash.old("title").await;    // Option<String>
errors.first("title");                        // Option<&str>
```

## Plain (unvalidated) bodies

When you do not need rules, the bare `Json<T>` / `Form<T>` extractors deserialise the
body directly. You can still return an error by hand:

```rust
#[post("/")]
async fn store(db: Db, Json(input): Json<CreateTodo>) -> Result<(StatusCode, Json<Todo>)> {
    if input.title.trim().is_empty() {
        return Err(Error::bad_request("title must not be empty"));
    }
    let todo = Todo::new(input.title.trim()).insert(&db).await?;
    Ok((StatusCode::CREATED, Json(todo)))
}
```

## The `ValidationErrors` type

Both the extractor and manual checks produce a `ValidationErrors` map (field → messages):

| Method | Returns |
|---|---|
| `is_empty()` | `true` when nothing failed |
| `has(field)` | whether a field has any error |
| `first(field)` | first message for a field, `Option<&str>` |
| `get(field)` | all messages for a field, `&[String]` |
| `all()` | every message, flattened |

Messages are plain English ("The title field is required.") and will become translatable
through the i18n package.
