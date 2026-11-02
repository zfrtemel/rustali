# Yanıtlar & Hatalar

## Bir yanıt döndürmek

Herhangi bir handler, `IntoResponse`'ı uygulayan bir tip döndürür. Yaygın olanlar:

| Dönüş tipi | Dönüştüğü |
|---|---|
| `&'static str`, `String` | `text/plain` |
| `Json<T>` | `application/json` |
| `Html<String>`, `View<T>` | `text/html` |
| `Redirect::to("/path")` | 303 yönlendirme |
| `StatusCode` | o durumla boş bir yanıt |
| `(StatusCode, T)` | `T`'nin yanıtı, durumu geçersiz kılınmış |
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

Tuple'lar bir yanıt oluşturur: başa bir durum, ortaya bir başlık dizisi ekle. Bu düz axum'dur —
herhangi bir `IntoResponse` tuple şekli çalışır:

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

Bir yönlendirme için `Redirect::to("/path")` (303) kullan; ihtiyaç duyduğunda
`Redirect::permanent(...)` ve `Redirect::temporary(...)` 308 / 307 verir.

## Hatalar değerdir

İstisna yoktur. Başarısız olabilen iş `Result<T>` döndürür — `Result<T, rustali::Error>`'ın
takma adı — ve `?` başarısızlıkları yayar. Framework, bir `Error`'ı senin için doğru HTTP
yanıtına çevirir.

```rust
#[get("/{id}")]
async fn show(db: Db, Path(id): Path<i64>) -> Result<Json<Todo>> {
    // find_or_404 eksikse Err(Error::NotFound) döndürür; `?` onu bir 404'e çevirir
    Ok(Json(Todo::find_or_404(&db, id).await?))
}
```

### Hata tipi

`rustali::Error`, her varyantı bir HTTP durumuna eşler:

| Varyant | Durum | Kurucu |
|---|---|---|
| `NotFound` | 404 | `Error::NotFound` |
| `BadRequest(msg)` | 400 | `Error::bad_request("...")` |
| `Unauthorized` | 401 | `Error::Unauthorized` |
| `Forbidden` | 403 | `Error::Forbidden` |
| `Validation(errors)` | 422 | [doğrulanmış](validation) extractor'lardan |
| `TooManyRequests { .. }` | 429 | `Throttle`'dan |
| `Database(..)` | 500 | `Error::database(e)` (ORM `DbErr`'i senin için eşler) |
| `Config(..)` | 500 | yapılandırma yükleme başarısızlığı |
| `Internal(..)` | 500 | `Error::internal(e)` — diğer herhangi bir hata |

`?` yaygın hataları otomatik olarak dönüştürür: `std::io::Error` ve `figment::Error`'ın `From`
uygulamaları vardır ve ORM, SeaORM'ün `DbErr`'ini eşler (eksik bir satır `NotFound` olur, geri
kalanı `Database`). Yalnızca `BadRequest` ve `Validation`, detaylarını istemcilere gösterir;
`Internal`/`Database` detayları **yalnızca `app.debug` açıkken** gösterilir; böylece üretim asla
iç detayları sızdırmaz.

### Bilerek bir hata döndürmek

Bir kurucu ve `?` ile birini yükselt ya da `return Err(...)`:

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

Bu ayrım bilinçlidir: `bad_request`, **istemci** hataları içindir; bu yüzden mesajı çağıran
tarafından okunmak üzere tasarlanmıştır ve her zaman gönderilir. `Forbidden` / `Unauthorized`
bilerek hiç detay taşımaz — bir çağırana *neden* reddedildiğini söylemek başlı başına bir
sızıntıdır. Ve `internal` / veritabanı hataları detaylarını debug dışında gizler; böylece bir
yığın mesajı ya da SQL dizesi, senin için request id ile loglanırken asla bir üretim istemcisine
ulaşmaz.

## Yanıt zarfı

Bir JSON API için, başarılı yanıtları `ApiResponse<T>` içine sar; böylece her yanıt — başarı
**ve** hata — **aynı üç alana** sahip olur: `success` / `data` / `error`. Bir istemci tek bir
şekli ayrıştırır ve hangi anahtarların mevcut olduğuna göre asla dallanmaz.

```rust
use rustali::prelude::*;

#[get("/api/todos/{id}")]
async fn show(db: Db, Path(id): Path<i64>) -> Result<ApiResponse<Todo>> {
    let todo = Todo::find_or_404(&db, id).await?;   // bir eksik yine hata zarfı olur
    Ok(ApiResponse::ok(todo))
}
```

Bir başarı şöyle serileşir:

```json
{ "success": true, "data": { "id": 1, "title": "write docs" }, "error": null }
```

Hatalar **aynı** zarfa otomatik olarak render edilir — onları elle oluşturmazsın. Yukarıdaki
`Todo::find_or_404`, `Err(Error::NotFound)` döndürür ve hata katmanı onu şuna çevirir:

```json
{ "success": false, "data": null, "error": { "code": 404, "message": "Not Found", "details": null } }
```

Şekil asla değişmez: `data` başarısızlıkta `null`, `error` başarıda `null` olur ve üç anahtar her
zaman mevcuttur. Bir hataya özgü her şey — alan başına doğrulama mesajları, request id, yol —
fazladan bir üst düzey anahtar olarak değil, `error.details` içinde yaşar:

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

Yani istemci kodu basitçe şöyledir: `success` ise `data`'yı oku; aksi hâlde `error`'ı oku.

## İçerik pazarlığı (content negotiation)

Aynı hata, kimin sorduğuna bağlı olarak farklı render edilir — tek bir controller'ın SSR'yi ve
bir API'yi sunmasını sağlayan şey budur. Kural:

- `Accept`, `text/html` içeriyorsa → bir **HTML hata sayfası**.
- Aksi hâlde, `Accept`, `json`'dan bahsediyorsa **ya da** yol `/api/` ile başlıyorsa → yukarıdaki
  JSON zarfı.

Eksik bir sayfaya ulaşan bir tarayıcı, stillenmiş bir 404 sayfası alır; `/api/...`'ya bir
`fetch()`, `{ success, data, error }` zarfını alır. Her yanıt ayrıca logları ilişkilendirmek
için bir `x-request-id` başlığı taşır (başarısızlıklarda `error.details.request_id`'ye
yansıtılır).

## Özel hata sayfaları

Render'ı `App::builder().on_error(...)` ile geçersiz kıl —
[Middleware & Filtreler](middleware#hata-render-ini-geçersiz-kılmak)'e bak. Hatayı ve bazı istek
bilgilerini alır ve kendi yanıtını döndürebilir ya da varsayılana erteleyebilir.

## Panikler

Bir handler'daki bir panik yakalanır ve bir `500`'e çevrilir (varsayılan yığının catch-panic
katmanı); böylece bir kötü istek asla sunucuyu düşürmez. Bir panik yine de bir hatadır — onu
düzelt; yakalama bir güvenlik ağıdır, hata işleme değil.
