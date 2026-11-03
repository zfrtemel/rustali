# Kimlik Doğrulama

Kimlik doğrulama kutudan çıktığı gibi hazırdır: argon2id ile parola özetleme, senin uyguladığın bir
kullanıcı sözleşmesi ve request'e bağlanan bir guard zinciri (SSR için çerez session'ları, API'ler
için bearer JWT). Böylece `Auth<User>` hiç uğraşmadan çalışır.

## Parçalar

1. **`Authenticatable`** — kullanıcı tipin bunu uygular.
2. **`UserStore`** — kullanıcıları yüklemek için bunu sen uygularsın.
3. **`AuthProvider`** — depoyu ve guard zincirini kaydeder.

## 1. Kullanıcın

Herhangi bir tip kullanıcı olabilir; çoğunlukla bir `#[entity]` modelidir.
`Authenticatable`'ı uygula:

```rust
use rustali::prelude::*;

#[entity]
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

`auth_id`, session'larda ve token'larda saklanan tanımlayıcıdır; `password_hash`, kayıtlı argon2 PHC
dizesidir; `roles` ise yerleşik `role:<name>` yeteneğini besler ([Yetkilendirme](authorization)).

## 2. Kullanıcı deposu

Kullanıcıları yüklemek için `UserStore`'u uygula — hem id'ye göre (bir session'ı veya token'ı geri
yüklemek için) hem de giriş alanına göre (kimlik bilgilerini doğrulamak için):

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

Auth, session'lara ihtiyaç duyar; bu yüzden `SessionProvider`'ı ve session katmanlarını da ekle:

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

Guard zinciri varsayılan olarak `[Session, Bearer]`'dir — session çerezi ya da bir
`Authorization: Bearer <jwt>` başlığı bir kullanıcıya çözülürse request kimliği doğrulanmış sayılır.
Guard'lar sırayla denenir ve ilk çözülen kazanır; böylece tek bir uygulama hem bir SSR sitesini
(çerezler) hem de bir API'yi (token'lar) aynı handler'lar üzerinden sunabilir. Zinciri
`AuthProvider::new(Users).guards([Guard::Session])` ile değiştir — örneğin yalnızca çerez kullanan bir
uygulama ya da session guard'ını büsbütün çıkaran, yalnızca API sunan bir servis için. `Bearer`
guard'ı `jwt` özelliğine ihtiyaç duyar (varsayılan olarak açık).

## Parolalar

Parolayı kayıt sırasında özetle, hiçbir zaman düz metin saklama:

```rust
let hash = password::hash(&input.password)?;        // argon2id PHC dizesi
let user = User::new(&name, &email, hash).insert(&db).await?;
```

`password::hash`, bir argon2id **PHC dizesi** üretir — algoritmayı, parametreleri ve her parola için
rastgele bir tuzu kendi içine gömer; böylece yalnızca o tek `password_hash` kolonunu saklaman yeterli
olur, ayrı bir tuz tutman gerekmez. Doğrulama sabit zamanlıdır; ya `Login::attempt` (aşağıda) senin
yerine yapar ya da `password::verify(plain, &hash)?` ile elle yaparsın. İkisi de `Result` döndürür:
bozuk biçimli bir özet sessizce `false` dönmez, hata fırlatır; böylece bir bozulma herkesi sessizce
kilitlemek yerine açığa çıkar.

## Kayıt olma ve giriş yapma

`Login<User>` extractor'ı session'a girişi halleder (session fixation saldırısını önlemek için
session id'sini yeniler):

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

`Login` metotları: `attempt(user, password)` → `bool`, `login(user)` (parola doğrulaması yapmaz),
`logout()`.

## Mevcut kullanıcıyı okumak

| Extractor | Davranış |
|---|---|
| `Auth<User>` | Kullanıcıyı verir; yoksa `401` (API) / işlenmiş hata sayfası (SSR). |
| `OptionalAuth<User>` | `Option<User>`; hiçbir zaman başarısız olmaz. |
| `Guest<User>` | Biri giriş yapmış *ise* `403` ile reddeder (örneğin giriş sayfası için). |

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

API istemcileri için `Bearer` guard'ı, `app.key` ile imzalanmış bir HS256 JWT'yi doğrular. Kimlik
bilgilerini doğruladıktan sonra bir token ver ve onu döndür:

```rust
use rustali::auth_kit::jwt;

let token = jwt::issue(&ctx, &user.auth_id(), 3600)?;   // 1 saat geçerli
```

İstemciler bundan sonra `Authorization: Bearer <token>` gönderir; `Auth<User>` bu token'ı, aynen bir
session gibi, depo üzerinden çözer. `app.key` en az 16 karakter olmalıdır
([Yapılandırma](../getting-started/configuration)'ya bak).

> Akışın tamamı — kayıt, giriş, çıkış, korunan bir sayfa ve yanlış parola durumu — `auth-app`
> örneğinde uçtan uca test edilir.
