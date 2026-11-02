# Yönlendirme & Controller

Rustali'de bir route, **ona hizmet veren handler'ın üzerinde** yaşar. Ayrı bir route dosyası
tutmazsın; bir metodu HTTP fiili ve yoluyla işaretlersin ve `#[controller]` makrosu onu
router'a bağlar — route ve ona yanıt veren kod bir arada durur ve `cargo run -- routes` tüm
tabloyu yazdırır.

## İlk controller

```rust
use rustali::prelude::*;

pub struct HomeController;

#[controller("/")]
impl HomeController {
    #[get("/", name = "home")]
    async fn home() -> &'static str {
        "Welcome to Rustali!"
    }
}
```

Dikkat edilecek üç şey:

- `#[controller("/")]` bir **inherent `impl`** üzerine gelir ve içindeki her route için bir
  yol öneki bildirir.
- `#[get("/")]` bir route bildirir. Yolu, controller önekiyle **birleştirilir**; bu yüzden bu
  handler `GET /`'e yanıt verir.
- Handler'lar **`self` olmayan** düz `async fn`'lerdir. Bir controller durumsuz bir ad
  uzayıdır; bir handler'ın ihtiyacı olan her şey extractor'lar aracılığıyla gelir (aşağıya
  bak). Bu derleme zamanında zorunlu kılınır — bir `self` alıcısı net bir hatadır.

Uygulamayı kurarken bağla:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    App::builder().controllers().run().await
}
```

`controllers()`, binary'ye bağlanmış **her** `#[controller]`'ı bağlar — senkronize tutulacak
bir liste yoktur. Bunun yerine birini açıkça bağlamak için
`App::builder().controller::<HomeController>()` kullan.

## HTTP fiilleri

Her metot attribute'u bir fiile eşlenir: `#[get]`, `#[post]`, `#[put]`, `#[patch]`,
`#[delete]`, `#[head]`, `#[options]`. Birkaç metot bir yolu paylaşabilir:

```rust
#[controller("/api/todos")]
impl TodoController {
    #[get("/")]              async fn index(/* ... */) { /* ... */ }
    #[post("/")]             async fn store(/* ... */) { /* ... */ }
    #[get("/{id}")]          async fn show(/* ... */)  { /* ... */ }
    #[patch("/{id}")]        async fn update(/* ... */) { /* ... */ }
    #[delete("/{id}")]       async fn destroy(/* ... */) { /* ... */ }
}
```

## Yol parametreleri

Dinamik segmentler `{name}` (axum söz dizimi) kullanır ve `Path` extractor'ıyla çıkarılır:

```rust
#[get("/{id}")]
async fn show(db: Db, Path(id): Path<i64>) -> Result<Json<Todo>> {
    Ok(Json(Todo::find_or_404(&db, id).await?))
}
```

Birkaç segment için, bildirim sırasına göre bir tuple çıkar:

```rust
#[get("/{year}/{slug}")]
async fn show(Path((year, slug)): Path<(i64, String)>) -> String {
    format!("{year}: {slug}")
}
```

Ya da ikiden fazlasında daha okunaklı olan, onları ada göre bir `Deserialize` struct'a bağla:

```rust
#[derive(Deserialize)]
struct PostPath { year: i64, slug: String }

#[get("/{year}/{slug}")]
async fn show(Path(p): Path<PostPath>) -> String {
    format!("{}: {}", p.year, p.slug)
}
```

## Sorgu dizeleri

Sorgu dizesini `Query` ile bir `Deserialize` struct'a bağla:

```rust
#[derive(Deserialize)]
struct Paging {
    page: Option<u64>,
    per_page: Option<u64>,
}

#[get("/")]
async fn index(db: Db, Query(paging): Query<Paging>) -> Result<Json<Page<Todo>>> {
    let page = Todo::query()
        .order_by_desc(Todo::ID)
        .paginate(&db, paging.page.unwrap_or(1), paging.per_page.unwrap_or(20))
        .await?;
    Ok(Json(page))
}
```

## İstek gövdeleri

`Json<T>` bir JSON gövdesini ayrıştırır; `Form<T>` bir URL kodlu form gövdesini ayrıştırır.
Doğrulamak istediğin girdiler için bunun yerine `Validated<Json<T>>` / `Validated<Form<T>>`'e
başvur — [İstekler & Doğrulama](validation.md)'ya bak.

```rust
#[post("/")]
async fn store(db: Db, Json(input): Json<CreateTodo>) -> Result<(StatusCode, Json<Todo>)> {
    let todo = Todo::new(input.title.trim()).insert(&db).await?;
    Ok((StatusCode::CREATED, Json(todo)))
}
```

## Yaygın extractor'lar

axum'un `FromRequestParts`/`FromRequest`'ini uygulayan her şey bir handler argümanı olabilir.
En çok kullanacakların:

| Extractor | Sana verdiği |
|---|---|
| `Path<T>` | Yol parametreleri |
| `Query<T>` | Ayrıştırılmış sorgu dizesi |
| `Json<T>` / `Form<T>` | İstek gövdesi |
| `Db` | Bir veritabanı bağlantısı (`orm` özelliği) |
| `Ctx` | Paylaşılan `AppContext` (yapılandırma, servisler) |
| `Inject<T>` | Tipe göre tek bir kayıtlı servis |
| `Auth<U>` / `OptionalAuth<U>` | Kimliği doğrulanmış kullanıcı (`auth` özelliği) |
| `Flash`, `Csrf`, `Session` | Oturum yardımcıları (`sessions` özelliği) |

Extractor'lar iki türe ayrılır ve bu ayrım akılda tutulacak tek kuraldır:

- **İstek başlığından** (`Path`, `Query`, `Ctx`, `Db`, `Inject`, `Auth`, `Session`, …) yalnızca
  üst veri okur; bu yüzden istediğin kadarına, herhangi bir sırada sahip olabilirsin.
- **Gövdeden** (`Json`, `Form`, `Validated`) isteği **tüketir**; bu yüzden en fazla bir tane
  olabilir ve argüman listesinde **en sona** gelmelidir.

```rust
#[post("/{id}/comments")]
async fn add_comment(
    db: Db,                               // başlık
    Auth(user): Auth<User>,               // başlık
    Path(id): Path<i64>,                  // başlık
    Json(input): Json<NewComment>,        // gövde — en sonda olmalı
) -> Result<(StatusCode, Json<Comment>)> {
    // ...
}
```

Bir gövde extractor'ını en son dışında bir yere koymak derleme hatasıdır; yani kural yalnızca
tavsiye edilmez, zorunlu kılınır.

## Yanıtlar

Bir handler, `IntoResponse`'ı uygulayan her şeyi döndürür:

- `&'static str` / `String` → `text/plain`
- `Json<T>` → `application/json`
- `Html<String>` ya da `View<T>` → `text/html` ([Görünümler](views.md)'e bak)
- `Redirect::to("/path")` → 303 yönlendirme
- `StatusCode` ya da `(StatusCode::CREATED, Json(todo))` gibi bir tuple ile durumu ayarla
- Hatanın framework [`Error`](responses.md)'ı olduğu `Result<T>` — `?` sorunsuz çalışır

```rust
#[delete("/{id}")]
async fn destroy(db: Db, Path(id): Path<i64>) -> Result<StatusCode> {
    Todo::find_or_404(&db, id).await?.delete(&db).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

## Adlandırılmış route'lar

Bir route'a `name` ver; route tablosunda ve (görünüm katmanının URL yardımcısı geldiğinde)
üretilen bağlantılarda görünür:

```rust
#[get("/{id}", name = "todos.show")]
async fn show(/* ... */) { /* ... */ }
```

Her controller route'unu, adını, handler'ını ve middleware'ini listele:

```bash
cargo run -- routes
```

```text
METHOD  PATH             NAME          HANDLER                 MIDDLEWARE
GET     /api/todos       todos.index   TodoController::index   Throttle::per_minute(600)
POST    /api/todos       todos.store   TodoController::store   Throttle::per_minute(600)
GET     /api/todos/{id}  todos.show    TodoController::show     Throttle::per_minute(600)
```

## Tek controller'dan SSR ve API

Bir handler yalnızca bir `IntoResponse` döndürdüğü için, aynı controller HTML sayfalarını ve
bir JSON API'yi yan yana sunabilir — her route için dönüş tipini seç:

```rust
#[controller("/posts")]
impl PostController {
    #[get("/")]            async fn index() -> Result<View<PostsIndex>> { /* HTML */ }
    #[get("/api/posts")]   async fn api_index() -> Result<Json<Vec<Post>>> { /* JSON */ }
}
```

Hatalar istemciye uyacak şekilde otomatik olarak render edilir (API çağıranlar için JSON,
tarayıcılar için bir HTML sayfası) — [Yanıtlar & Hatalar](responses.md)'a bak.

## Eşleşmeyen route'lar

Hiçbir route'la eşleşmeyen bir istek, framework'ün `NotFound` hatasını döndüren yerleşik bir
handler'a düşer; böylece bir 404, diğer herhangi bir hata gibi pazarlık edilir — bir API
istemcisi için JSON, bir tarayıcı için bir HTML hata sayfası. Bunu sen bağlamazsın; varsayılan
yığınla gelir.

## Middleware iliştirmek

Bir metottaki `#[middleware(expr)]` o tek handler'ı sarar; `impl` üzerindeki tüm controller'ı
sarar. [Middleware & Filtreler](middleware.md)'e bak.

```rust
#[controller("/admin")]
#[middleware(Throttle::per_minute(60))]   // tüm controller
impl AdminController {
    #[delete("/users/{id}")]
    #[authorize("user.delete")]            // tek route
    async fn destroy(/* ... */) { /* ... */ }
}
```

> **Kaputun altında:** `#[controller]` sıradan axum router kodu üretir — görmek için
> `cargo expand` çalıştır. Çalışma zamanı yansıması (reflection) yoktur; makro yalnızca route
> bildirimini handler'ın yanına taşır.
