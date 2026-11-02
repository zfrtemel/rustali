# Routing & Controller

Rustali'de bir route, **ona hizmet veren handler'ın hemen üstünde** durur. Ayrı bir route dosyası
tutmazsın; bir metodu HTTP metoduyla ve yoluyla işaretlersin, `#[controller]` makrosu da onu
router'a bağlar. Böylece route ile ona response veren kod yan yana durur ve `cargo run -- routes`
tüm tabloyu ekrana basar.

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

- `#[controller("/")]` bir **inherent `impl`** üzerine konur ve içindeki her route için ortak
  bir yol öneki tanımlar.
- `#[get("/")]` bir route tanımlar. Verdiğin yol, controller önekiyle **birleştirilir**; yani bu
  handler `GET /`'e response verir.
- Handler'lar `self` almayan düz `async fn`'lerdir. Bir controller, durum tutmayan bir ad
  alanıdır; bir handler'ın ihtiyaç duyduğu her şey ona extractor'lar üzerinden gelir (aşağıya
  bak). Bu kural derleme zamanında denetlenir — `self` alan bir handler doğrudan hata verir.

Uygulamayı kurarken şöyle bağlarsın:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    App::builder().controllers().run().await
}
```

`controllers()`, binary'ye dahil edilmiş **bütün** `#[controller]`'ları bağlar; elle güncel
tutman gereken bir liste yoktur. Bunun yerine tek bir controller'ı açıkça bağlamak istersen
`App::builder().controller::<HomeController>()` kullan.

## HTTP fiilleri

Her metot attribute'u bir HTTP metoduna karşılık gelir: `#[get]`, `#[post]`, `#[put]`, `#[patch]`,
`#[delete]`, `#[head]`, `#[options]`. Birden fazla metot aynı yolu paylaşabilir:

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

Dinamik segmentler `{name}` biçimini (axum söz dizimi) kullanır ve `Path` extractor'ı ile alınır:

```rust
#[get("/{id}")]
async fn show(db: Db, Path(id): Path<i64>) -> Result<Json<Todo>> {
    Ok(Json(Todo::find_or_404(&db, id).await?))
}
```

Birden fazla segment için, segmentleri bildirim sırasına göre bir tuple'a alırsın:

```rust
#[get("/{year}/{slug}")]
async fn show(Path((year, slug)): Path<(i64, String)>) -> String {
    format!("{year}: {slug}")
}
```

İkiden fazla segment varsa daha okunaklı olan yöntem, segmentleri ada göre bir `Deserialize`
struct'a bağlamaktır:

```rust
#[derive(Deserialize)]
struct PostPath { year: i64, slug: String }

#[get("/{year}/{slug}")]
async fn show(Path(p): Path<PostPath>) -> String {
    format!("{}: {}", p.year, p.slug)
}
```

## Query dizeleri

Query dizesini `Query` ile bir `Deserialize` struct'a bağlarsın:

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

## Request gövdeleri

`Json<T>` bir JSON gövdesini, `Form<T>` ise URL kodlu bir form gövdesini ayrıştırır.
Validation'dan geçirmek istediğin girdiler için bunların yerine `Validated<Json<T>>` /
`Validated<Form<T>>` kullan — [Request'ler & Validation](validation.md)'a bak.

```rust
#[post("/")]
async fn store(db: Db, Json(input): Json<CreateTodo>) -> Result<(StatusCode, Json<Todo>)> {
    let todo = Todo::new(input.title.trim()).insert(&db).await?;
    Ok((StatusCode::CREATED, Json(todo)))
}
```

## Yaygın extractor'lar

axum'un `FromRequestParts`/`FromRequest`'ini uygulayan her tip bir handler argümanı olabilir.
En sık kullanacakların şunlar:

| Extractor | Sana verdiği |
|---|---|
| `Path<T>` | Yol parametreleri |
| `Query<T>` | Ayrıştırılmış query dizesi |
| `Json<T>` / `Form<T>` | Request gövdesi |
| `Db` | Veritabanı bağlantısı (`orm` özelliği) |
| `Ctx` | Paylaşılan `AppContext` (yapılandırma, servisler) |
| `Inject<T>` | Tipe göre kayıtlı tek bir servis |
| `Auth<U>` / `OptionalAuth<U>` | Kimliği doğrulanmış kullanıcı (`auth` özelliği) |
| `Flash`, `Csrf`, `Session` | Session yardımcıları (`sessions` özelliği) |

Extractor'lar iki gruba ayrılır ve aklında tutman gereken tek kural bu ayrımdır:

- **Request başlığından okuyanlar** (`Path`, `Query`, `Ctx`, `Db`, `Inject`, `Auth`, `Session`, …)
  yalnızca üst veriyi okur; bu yüzden bunlardan kaç tane istersen, hangi sırada istersen
  kullanabilirsin.
- **Gövdeden okuyanlar** (`Json`, `Form`, `Validated`) request'i **tüketir**; bu yüzden en fazla
  bir tane olabilir ve argüman listesinde **en sonda** gelmelidir.

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

Bir gövde extractor'ını son sıra dışında bir yere koymak derleme hatasına yol açar; yani bu kural
sadece bir tavsiye değil, zorunluluktur.

## Response'lar

Bir handler, `IntoResponse`'ı uygulayan herhangi bir tipi döndürebilir:

- `&'static str` / `String` → `text/plain`
- `Json<T>` → `application/json`
- `Html<String>` ya da `View<T>` → `text/html` ([View'lar](views.md)'a bak)
- `Redirect::to("/path")` → 303 yönlendirme
- `StatusCode` ya da `(StatusCode::CREATED, Json(todo))` gibi bir tuple ile durum kodunu ayarlarsın
- Hata tipinin framework'ün [`Error`](responses.md)'ı olduğu `Result<T>` — `?` sorunsuz çalışır

```rust
#[delete("/{id}")]
async fn destroy(db: Db, Path(id): Path<i64>) -> Result<StatusCode> {
    Todo::find_or_404(&db, id).await?.delete(&db).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

## Adlandırılmış route'lar

Bir route'a `name` verirsin; bu ad hem route tablosunda hem de (view katmanının URL yardımcısı
geldiğinde) üretilen bağlantılarda görünür:

```rust
#[get("/{id}", name = "todos.show")]
async fn show(/* ... */) { /* ... */ }
```

Her controller route'unu adıyla, handler'ıyla ve middleware'iyle birlikte listelemek için:

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

Bir handler sadece bir `IntoResponse` döndürdüğü için, aynı controller hem HTML sayfalarını hem de
bir JSON API'yi bir arada sunabilir — her route için dönüş tipini kendin seçersin:

```rust
#[controller("/posts")]
impl PostController {
    #[get("/")]            async fn index() -> Result<View<PostsIndex>> { /* HTML */ }
    #[get("/api/posts")]   async fn api_index() -> Result<Json<Vec<Post>>> { /* JSON */ }
}
```

Hatalar istemciye göre otomatik olarak render edilir (API'yi çağıranlar için JSON, tarayıcılar
için bir HTML sayfası) — [Response'lar & Hatalar](responses.md)'a bak.

## Eşleşmeyen route'lar

Hiçbir route'la eşleşmeyen bir request, framework'ün `NotFound` hatasını döndüren yerleşik bir
handler'ına düşer; böylece 404 de diğer hatalar gibi içerik pazarlığından geçer — bir API
istemcisine JSON, bir tarayıcıya HTML hata sayfası döner. Bunu sen bağlamazsın; varsayılan
yığınla birlikte gelir.

## Middleware iliştirmek

Bir metodun üzerindeki `#[middleware(expr)]` yalnızca o handler'ı sarar; `impl` üzerindeki ise tüm
controller'ı sarar. [Middleware & Filtreler](middleware.md)'e bak.

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
> `cargo expand` çalıştırabilirsin. Çalışma zamanında reflection kullanılmaz; makro yalnızca route
> bildirimini handler'ın yanına taşır.
