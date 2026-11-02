# Kimlik Doğrulama

Kimlik doğrulama kullanıma hazır gelir: argon2id parola özetleme, uyguladığın bir kullanıcı
sözleşmesi ve isteğe bağlanmış bir guard zinciri (SSR için çerez session'ları, API'ler için bearer
JWT); böylece `Auth<User>` sorunsuz çalışır.

## Parçalar

1. **`Authenticatable`** — kullanıcı tipin onu uygular.
2. **`UserStore`** — kullanıcıları yüklemek için onu uygularsın.
3. **`AuthProvider`** — depoyu ve guard zincirini kaydeder.

## 1. Kullanıcın

Herhangi bir tip bir kullanıcı olabilir; genellikle bir `#[derive(Entity)]` modelidir.
`Authenticatable`'ı uygula:

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
    // fn roles(&self) -> Vec<String> { ... }   // isteğe bağlı; varsayılan hiçbiri
}
```

`auth_id`, session'larda ve token'larda saklanan tanımlayıcıdır; `password_hash`, saklanan argon2 PHC
dizesidir; `roles`, yerleşik `role:<name>` yeteneğini besler ([Yetkilendirme](authorization)).

## 2. Kullanıcı deposu

Kullanıcıları yüklemek için `UserStore`'u uygula — id'ye göre (bir session'ı/token'ı geri yüklemek
için) ve giriş alanına göre (kimlik bilgilerini kontrol etmek için):

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

## 3. Bağla

Auth'un session'lara ihtiyacı var; bu yüzden `SessionProvider`'ı ve session katmanlarını da ekle:

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

Varsayılan olarak guard zinciri `[Session, Bearer]`'dir — session çerezi ya da bir
`Authorization: Bearer <jwt>` bir kullanıcıya çözülürse bir request kimliği doğrulanmış olur. Guard'lar
sırayla denenir ve çözülen ilki kazanır; böylece tek bir uygulama hem bir SSR sitesini (çerezler)
hem de bir API'yi (token'lar) aynı handler'lardan sunar. Zinciri
`AuthProvider::new(Users).guards([Guard::Session])` ile değiştir — örneğin, yalnızca çerezli bir
uygulama ya da session guard'ını bırakan yalnızca API'li bir servis. `Bearer` guard'ı `jwt`
özelliğine ihtiyaç duyar (varsayılan olarak açık).

## Parolalar

Kayıtta özetle, asla düz metin saklama:

```rust
let hash = password::hash(&input.password)?;        // argon2id PHC dizesi
let user = User::new(&name, &email, hash).insert(&db).await?;
```

`password::hash`, bir argon2id **PHC dizesi** üretir — algoritmayı, parametreleri ve parola başına
rastgele bir tuzu gömer; böylece yalnızca o tek `password_hash` kolonunu saklarsın ve ayrı bir tuza
ihtiyacın olmaz. Doğrulama sabit zamanlıdır ve senin için `Login::attempt` (aşağıda) içinde ya da
`password::verify(plain, &hash)?` ile elle gerçekleşir. İkisi de `Result` döndürür: hatalı biçimli
bir saklanan özet sessiz bir `false` değil, bir hatadır; böylece bozulma, herkesi sessizce
kilitlemek yerine su yüzüne çıkar.

## Kayıt olma ve giriş yapma

`Login<User>` extractor'ı session girişini gerçekleştirir (fiksasyonu önlemek için session id'sini
döndürür):

```rust
#[post("/register")]
async fn register(
    db: Db,
    login: Login<User>,
    Validated(Form(input)): Validated<Form<RegisterForm>>,
) -> Result<Redirect> {
    let hash = password::hash(&input.password)?;
    let user = User::new(input.name.trim(), input.email.trim(), hash).insert(&db).await?;
    login.login(&user).await?;                 // onları hemen giriş yaptır
    Ok(Redirect::to("/dashboard"))
}

#[post("/login")]
async fn login(db: Db, login: Login<User>, Form(input): Form<LoginForm>) -> Result<Redirect> {
    let user = User::query().filter(User::EMAIL.eq(&input.email)).fetch_one(&db).await?;
    let ok = match &user {
        Some(user) => login.attempt(user, &input.password).await?,   // doğrula + giriş yap
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

`Login` metotları: `attempt(user, password)` → `bool`, `login(user)` (parola kontrolü yok),
`logout()`.

## Mevcut kullanıcıyı okumak

| Extractor | Davranış |
|---|---|
| `Auth<User>` | Kullanıcı ya da yoksa `401` (API) / işlenmiş hata (SSR). |
| `OptionalAuth<User>` | `Option<User>`; asla başarısız olmaz. |
| `Guest<User>` | Biri giriş yapmış *ise* `403` ile reddeder (örn. giriş sayfası). |

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

## API token'ları (JWT)

API istemcileri için, `Bearer` guard'ı `app.key` ile imzalanmış bir HS256 JWT'yi doğrular. Kimlik
bilgilerini kontrol ettikten sonra bir tane ver ve onu döndür:

```rust
use rustali::auth_kit::jwt;

let token = jwt::issue(&ctx, &user.auth_id(), 3600)?;   // 1 saat geçerli
```

İstemciler ardından `Authorization: Bearer <token>` gönderir; `Auth<User>`, onu tam olarak bir
session gibi depo aracılığıyla çözer. `app.key` en az 16 karakter olmalıdır
([Yapılandırma](../getting-started/configuration)'ya bak).

> Tüm akış — kayıt, giriş, çıkış, korunan bir sayfa ve yanlış-parola yolu — `auth-app` örneğinde
> uçtan uca denenir.
