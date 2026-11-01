# Authentication

Authentication ships ready to use: argon2id password hashing, a user contract you
implement, and a guard chain (cookie sessions for SSR, bearer JWT for APIs) wired into the
request so `Auth<User>` just works.

## The pieces

1. **`Authenticatable`** — your user type implements it.
2. **`UserStore`** — you implement it to load users.
3. **`AuthProvider`** — registers the store and the guard chain.

## 1. Your user

Any type can be a user; usually it is a `#[derive(Entity)]` model. Implement
`Authenticatable`:

```rust
use rustali::prelude::*;

#[derive(Entity, Debug, Clone, Serialize)]
#[table("users")]
pub struct User {
    pub id: i64,
    pub name: String,
    #[column(unique)]
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Authenticatable for User {
    fn auth_id(&self) -> String { self.id.to_string() }
    fn password_hash(&self) -> &str { &self.password_hash }
    // fn roles(&self) -> Vec<String> { ... }   // optional; default is none
}
```

`auth_id` is the identifier stored in sessions and tokens; `password_hash` is the stored
argon2 PHC string; `roles` feeds the built-in `role:<name>` ability
([Authorization](authorization)).

## 2. The user store

Implement `UserStore` to load users — by id (to restore a session/token) and by login
field (to check credentials):

```rust
pub struct Users;

impl UserStore for Users {
    type User = User;

    fn find_by_id<'a>(&'a self, ctx: &'a AppContext, id: &'a str) -> StoreFuture<'a, Option<User>> {
        Box::pin(async move {
            let Ok(id) = id.parse::<i64>() else { return Ok(None) };
            User::find(&*ctx.db()?, id).await
        })
    }

    fn find_by_login<'a>(&'a self, ctx: &'a AppContext, email: &'a str) -> StoreFuture<'a, Option<User>> {
        Box::pin(async move {
            User::query().filter(User::EMAIL.eq(email)).fetch_one(&*ctx.db()?).await
        })
    }
}
```

## 3. Wire it up

Auth needs sessions, so add `SessionProvider` and the session layers too:

```rust
App::builder()
    .provider(DatabaseProvider)
    .provider(SessionProvider)
    .provider(AuthProvider::new(Users))
    .configure(session_layers)
    .controllers()
    .run()
    .await
```

By default the guard chain is `[Session, Bearer]` — a request is authenticated if either
the session cookie or a `Authorization: Bearer <jwt>` resolves to a user. Change it with
`AuthProvider::new(Users).guards([Guard::Session])`.

## Passwords

Hash on registration, never store plaintext:

```rust
let hash = password::hash(&input.password)?;        // argon2id PHC string
let user = User::new(&name, &email, hash).insert(&db).await?;
```

Verification is constant-time and happens for you inside `Login::attempt` (below), or by
hand with `password::verify(plain, &hash)?`.

## Registering and logging in

The `Login<User>` extractor performs the session login (it rotates the session id to
prevent fixation):

```rust
#[post("/register")]
async fn register(
    db: Db,
    login: Login<User>,
    Validated(Form(input)): Validated<Form<RegisterForm>>,
) -> Result<Redirect> {
    let hash = password::hash(&input.password)?;
    let user = User::new(input.name.trim(), input.email.trim(), hash).insert(&db).await?;
    login.login(&user).await?;                 // log them in immediately
    Ok(Redirect::to("/dashboard"))
}

#[post("/login")]
async fn login(db: Db, login: Login<User>, Form(input): Form<LoginForm>) -> Result<Redirect> {
    let user = User::query().filter(User::EMAIL.eq(&input.email)).fetch_one(&db).await?;
    let ok = match &user {
        Some(user) => login.attempt(user, &input.password).await?,   // verify + log in
        None => false,
    };
    if ok { Ok(Redirect::to("/dashboard")) } else { Err(Error::Unauthorized) }
}

#[post("/logout")]
async fn logout(login: Login<User>) -> Result<Redirect> {
    login.logout().await?;
    Ok(Redirect::to("/login"))
}
```

`Login` methods: `attempt(user, password)` → `bool`, `login(user)` (no password check),
`logout()`.

## Reading the current user

| Extractor | Behaviour |
|---|---|
| `Auth<User>` | The user, or `401` (API) / handled error (SSR) when absent. |
| `OptionalAuth<User>` | `Option<User>`; never fails. |
| `Guest<User>` | Rejects with `403` if someone *is* logged in (e.g. the login page). |

```rust
#[get("/dashboard")]
async fn dashboard(Auth(user): Auth<User>) -> Result<View<Dashboard>> {
    Ok(View::new(Dashboard { name: user.name }))
}

#[get("/")]
async fn home(auth: OptionalAuth<User>) -> Redirect {
    if auth.is_some() { Redirect::to("/dashboard") } else { Redirect::to("/login") }
}
```

## API tokens (JWT)

For API clients the `Bearer` guard verifies an HS256 JWT signed with `app.key`. Issue one
after checking credentials and return it:

```rust
use rustali::auth_kit::jwt;

let token = jwt::issue(&ctx, &user.auth_id(), 3600)?;   // valid 1 hour
```

Clients then send `Authorization: Bearer <token>`; `Auth<User>` resolves it through the
store exactly like a session. `app.key` must be at least 16 characters (see
[Configuration](../getting-started/configuration)).

> The whole flow — register, login, logout, a protected page and a wrong-password path —
> is exercised end to end in the `auth-app` example.
