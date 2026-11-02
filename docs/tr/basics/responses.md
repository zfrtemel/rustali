# Response'lar & Hatalar

## Bir response döndürmek

Herhangi bir handler, `IntoResponse`'ı uygulayan bir tip döndürür. Yaygın olanlar şunlar:

| Dönüş tipi | Dönüştüğü |
|---|---|
| `&'static str`, `String` | `text/plain` |
| `Json<T>` | `application/json` |
| `Html<String>`, `View<T>` | `text/html` |
| `Redirect::to("/path")` | 303 yönlendirme |
| `StatusCode` | o durum koduyla boş bir response |
| `(StatusCode, T)` | `T`'nin response'u, durum kodu değiştirilmiş hâlde |
| `ApiResponse<T>` | standart `{ success, data, error }` JSON zarfı (aşağıya bak) |
| `Result<T>` | `Ok`'ta `T`, `Err`'de render edilmiş bir hata |

```rust
#[post("/")]
async fn store(db: Db, Json(input): Json<CreateTodo>) -> Result<(StatusCode, Json<Todo>)> {
    let todo = Todo::new(input.title.trim()).insert(&db).await?;
    Ok((StatusCode::CREATED, Json(todo)))
}
```

### Durum ve başlık ayarlamak

Response'u bir tuple ile de oluşturabilirsin: başa bir durum kodu, ortaya bir başlık dizisi
koyarsın. Bu doğrudan axum davranışıdır — `IntoResponse`'ı uygulayan her tuple şekli çalışır:

```rust
use rustali::http::{header, StatusCode};

#[get("/report.csv")]
async fn report() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/csv"),
         (header::CONTENT_DISPOSITION, "attachment; filename=\"report.csv\"")],
        csv_body,
    )
}
```

Yönlendirme için `Redirect::to("/path")` (303) kullan; gerektiğinde `Redirect::permanent(...)` ve
`Redirect::temporary(...)` sırasıyla 308 / 307 döndürür.

## Hatalar değerdir

Burada istisna (exception) yoktur. Başarısız olabilecek işler `Result<T>` döndürür — bu,
`Result<T, rustali::Error>`'ın takma adıdır — ve `?` operatörü hataları yukarı taşır. Framework, bir
`Error`'ı senin için doğru HTTP response'una çevirir.

```rust
#[get("/{id}")]
async fn show(db: Db, Path(id): Path<i64>) -> Result<Json<Todo>> {
    // find_or_404 eksikse Err(Error::NotFound) döndürür; `?` onu bir 404'e çevirir
    Ok(Json(Todo::find_or_404(&db, id).await?))
}
```

### Hata tipi

`rustali::Error`, her varyantını bir HTTP durum koduna eşler:

| Varyant | Durum | Kurucu |
|---|---|---|
| `NotFound` | 404 | `Error::NotFound` |
| `BadRequest(msg)` | 400 | `Error::bad_request("...")` |
| `Unauthorized` | 401 | `Error::Unauthorized` |
| `Forbidden` | 403 | `Error::Forbidden` |
| `Validation(errors)` | 422 | [validation'dan geçen](validation) extractor'lardan |
| `TooManyRequests { .. }` | 429 | `Throttle`'dan |
| `Database(..)` | 500 | `Error::database(e)` (ORM `DbErr`'i senin için eşler) |
| `Config(..)` | 500 | yapılandırma yükleme hatası |
| `Internal(..)` | 500 | `Error::internal(e)` — diğer tüm hatalar |

`?` yaygın hataları otomatik olarak dönüştürür: `std::io::Error` ve `figment::Error` için `From`
uygulamaları vardır; ORM ise SeaORM'ün `DbErr`'ini eşler (eksik bir satır `NotFound`, geri kalanı
`Database` olur). Detaylarını istemciye gösteren tek hatalar `BadRequest` ve `Validation`'dır;
`Internal`/`Database` detayları **yalnızca `app.debug` açıkken** gösterilir; böylece üretimde iç
detaylar asla dışarı sızmaz.

### Bilerek bir hata döndürmek

Bir kurucu ve `?` ile bir hata fırlat ya da `return Err(...)` kullan:

```rust
#[post("/transfer")]
async fn transfer(db: Db, Json(input): Json<Transfer>) -> Result<StatusCode> {
    if input.amount <= 0 {
        return Err(Error::bad_request("amount must be positive"));  // 400, detay gösterilir
    }
    let account = Account::find_or_404(&db, input.from).await?;      // eksikse 404
    if account.balance < input.amount {
        return Err(Error::Forbidden);                               // 403, detay sızdırılmaz
    }
    // ...
    Ok(StatusCode::NO_CONTENT)
}
```

Bu ayrım bilinçli yapılmıştır: `bad_request`, **istemci** hataları içindir; mesajı çağıran taraf
okusun diye yazılır ve her zaman gönderilir. `Forbidden` / `Unauthorized` ise bilerek hiç detay
taşımaz — bir çağırana *neden* reddedildiğini söylemek başlı başına bir sızıntıdır. `internal` ve
veritabanı hataları da detaylarını debug dışında gizler; böylece bir yığın izi (stack trace) ya da
bir SQL dizesi, senin için request id ile birlikte loglanırken üretimde hiçbir istemciye
ulaşmaz.

## Response zarfı

Bir JSON API'de başarılı response'ları `ApiResponse<T>` içine sar; böylece her response — başarı
**da** hata **da** — **aynı üç alanı** taşır: `success` / `data` / `error`. İstemci tek bir şekli
ayrıştırır ve hangi anahtarların bulunduğuna göre dallanmak zorunda kalmaz.

```rust
use rustali::prelude::*;

#[get("/api/todos/{id}")]
async fn show(db: Db, Path(id): Path<i64>) -> Result<ApiResponse<Todo>> {
    let todo = Todo::find_or_404(&db, id).await?;   // bir eksik yine hata zarfı olur
    Ok(ApiResponse::ok(todo))
}
```

Başarılı bir response şöyle serileşir:

```json
{ "success": true, "data": { "id": 1, "title": "write docs" }, "error": null }
```

Hatalar da **aynı** zarfa otomatik olarak render edilir — bunları elle oluşturman gerekmez.
Yukarıdaki `Todo::find_or_404`, `Err(Error::NotFound)` döndürür ve hata katmanı bunu şuna çevirir:

```json
{ "success": false, "data": null, "error": { "code": 404, "message": "Not Found", "details": null } }
```

Şekil hiç değişmez: başarısızlıkta `data`, başarıda `error` `null` olur ve üç anahtar her zaman
yerinde durur. Bir hataya özgü her şey — alan başına validation mesajları, request id, yol —
fazladan bir üst düzey anahtar olarak değil, `error.details` içinde durur:

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": 422,
    "message": "The given data was invalid.",
    "details": { "fields": { "title": ["The title field is required."] },
                 "request_id": "a1b2c3d4-...", "instance": "/api/todos" }
  }
}
```

Yani istemci kodu gayet basit olur: `success` doğruysa `data`'yı oku, değilse `error`'ı oku.

## İçerik pazarlığı (content negotiation)

Aynı hata, isteği kimin yaptığına göre farklı render edilir — tek bir controller'ın hem SSR hem de
bir API sunabilmesini sağlayan şey budur. Kural şu:

- `Accept` başlığı `text/html` içeriyorsa → bir **HTML hata sayfası**.
- Aksi hâlde `Accept` `json`'dan söz ediyorsa **ya da** yol `/api/` ile başlıyorsa → yukarıdaki
  JSON zarfı.

Var olmayan bir sayfaya giden bir tarayıcı stillenmiş bir 404 sayfası alır; `/api/...`'ya atılan
bir `fetch()` ise `{ success, data, error }` zarfını alır. Her response ayrıca logları
ilişkilendirebilmek için bir `x-request-id` başlığı taşır (başarısızlıklarda
`error.details.request_id` alanına da yansıtılır).

## Özel hata sayfaları

Render'ı `App::builder().on_error(...)` ile geçersiz kılarsın —
[Middleware & Filtreler](middleware#hata-render-ını-gecersiz-kılmak)'e bak. Bu closure, hatayı ve
bazı request bilgilerini alır; ister kendi response'unu döndürür, ister işi varsayılana bırakır.

## Panikler

Bir handler'da oluşan panik yakalanır ve bir `500`'e çevrilir (varsayılan yığının catch-panic
katmanı); böylece hatalı bir request sunucuyu asla düşürmez. Ama panik yine de bir hatadır — onu
düzelt; bu yakalama mekanizması bir güvenlik ağıdır, hata yönetimi yöntemi değil.
