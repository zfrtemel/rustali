# Middleware & Filtreler

Middleware, request'i girişte ve response'u çıkışta sarar. Rustali'de middleware aslında sadece bir
[tower] `Layer`'ıdır ve onu üç düzeyde iliştirebilirsin — global, controller başına ya da route
başına. Böylece pek çok yeri ilgilendiren ortak işler (hız sınırlama, CORS, auth, loglama)
handler'larının dışında kalır. Framework'ün tasarımındaki katmanlı filtre hattı budur.

[tower]: https://docs.rs/tower

## Varsayılan yığın

Her uygulama, en dıştan en içe şu varsayılan yığınla sarılır:

1. **Request ID** — bir `x-request-id` (UUID) atar ve bunu aktarır.
2. **Tracing** — `tracing` üzerinden yapılandırılmış request/response loglaması.
3. **Catch-panic** — panikleyen bir handler, bağlantıyı düşürmek yerine `500`'e dönüşür.
4. **Bağlam enjeksiyonu** — düz tower katmanlarının servislere erişebilmesi için `AppContext`'i
   request uzantılarına koyar.
5. **Hata pazarlığı** — bir framework [`Error`](responses)'ını, istemciye göre ya JSON'a ya da bir
   HTML sayfasına çevirir ("istisna filtresi").
6. **Senin katmanların** — aşağıda eklediğin her şey.
7. **Route'lar.**

Bunları sen elle eklemezsin; her zaman devrededirler.

## Global middleware

`App::builder().layer(...)` ile tüm route'ların çevresine bir katman eklersin:

```rust
use rustali::{middleware::Cors, prelude::*};

App::builder()
    .layer(Cors::permissive())
    .controllers()
    .run()
    .await
```

Senin global katmanların varsayılan yığının *içinde* yer alır; bu yüzden request id'yi yine
görürler ve hataları yine tutarlı biçimde render edilir.


### CORS

`Cors`, `tower_http`'nin yeniden dışa aktarılmış `CorsLayer`'ıdır. `Cors::permissive()` her kaynağa,
her metoda ve her başlığa izin verir — geliştirme sırasında kullanışlıdır, ama üretimde genellikle
tam olarak neye izin verdiğini açıkça belirtmek istersin:

```rust
use rustali::http::{HeaderValue, Method};
use rustali::middleware::Cors;

let cors = Cors::new()
    .allow_origin("https://app.example.com".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST]);

App::builder().layer(cors)
```

CORS'a yalnızca bir tarayıcı, API'ni farklı bir kaynaktan çağırdığında ihtiyacın olur; aynı
kaynaktan çalışan bir SSR uygulaması buna ihtiyaç duymaz.

## Controller ve route middleware'i

`impl` üzerindeki `#[middleware(expr)]` ile bütün bir controller'a, metot üzerindeki
`#[middleware(expr)]` ile de tek bir route'a katman iliştirirsin. Metot katmanları, controller
katmanlarının içinde çalışır.

```rust
#[controller("/api")]
#[middleware(Throttle::per_minute(600))]   // bu controller'daki her route
impl ApiController {
    #[get("/feed")]
    async fn feed() -> Result<Json<Vec<Post>>> { /* ... */ }

    #[post("/import")]
    #[middleware(Throttle::per_minute(5))]  // daha katı, yalnızca bu route
    async fn import() -> Result<StatusCode> { /* ... */ }
}
```

`expr`, tipi bir tower `Layer` olan herhangi bir değer olabilir — `Throttle` gibi bir framework
katmanı, `axum::middleware::from_fn(my_fn)` gibi bir axum yardımcısı ya da kendi yazdığın bir katman.

## Hız sınırlama

`Throttle`, sabit pencereli, bellek içinde çalışan bir hız sınırlayıcıdır. Bir
`x-ratelimit-remaining` başlığı ekler ve sınıra ulaşıldığında `Retry-After` başlığıyla birlikte
`429 Too Many Requests` response'u verir.

```rust
Throttle::per_minute(60)             // 60 istek / dakika
Throttle::per_second(10)             // 10 / saniye
Throttle::new(100, Duration::from_secs(300))   // 5 dakikada 100
```

İstemciler `X-Forwarded-For` başlığına (ilk adres) ya da eş IP'ye göre ayırt edilir. Bu, tek
süreç için geçerlidir; ilerleyen zamanda arkasına paylaşımlı bir depo (Redis) takılabilecek.

## Kendininkini yazmak

Herhangi bir axum middleware fonksiyonu kullanılabilir. Fonksiyonu `from_fn` ile sarıp uygun olan
düzeye eklersin:

```rust
use rustali::http::{extract::Request, middleware::{from_fn, Next}, response::Response};

async fn timing(request: Request, next: Next) -> Response {
    let start = std::time::Instant::now();
    let response = next.run(request).await;
    tracing::info!(elapsed = ?start.elapsed(), "handled");
    response
}

// global:
App::builder().layer(from_fn(timing))
```

`rustali make:middleware Timing` bu iskeleti senin için oluşturur.

### Kısa devre yapmak

Bir middleware'in `next.run(...)`'ı çağırması şart değil. `next.run`'dan önce bir response
döndürürsen handler hiç çalışmaz — bir gate'in, bir bakım anahtarının ya da bir başlık kontrolünün
bir request'i daha baştan reddetmesi tam olarak böyle olur:

```rust
use rustali::http::{
    extract::Request, middleware::Next, response::{IntoResponse, Response}, StatusCode,
};

async fn require_api_key(request: Request, next: Next) -> Response {
    match request.headers().get("x-api-key") {
        Some(key) if key == "expected" => next.run(request).await,
        _ => StatusCode::UNAUTHORIZED.into_response(),   // handler asla çalışmaz
    }
}
```

Bir middleware'in yapabileceği iki şey — `next.run`'dan önce request'e müdahale etmek ve
sonrasında response'a müdahale etmek — aslında her ihtiyacı karşılar: kimlik doğrulama, zamanlama,
başlık enjekte etme, response'ları yeniden yazma ve bir request'i doğrudan reddetme.

### Düzeyler nasıl iç içe geçer

Üç iliştirme noktası, dıştan içe doğru iç içe geçer: **global → controller → route → handler**.
Dıştaki katman request'i ilk, response'u ise en son görür. Yani global bir zamanlama katmanı
içindeki her şeyi ölçer, bir controller `Throttle`'ı bir route'un kendi daha katı `Throttle`'ından
önce sayar ve bir route `#[authorize]`'u handler'a en yakın noktada çalışır. İşi görecek en dar
düzeyi seç: gerçekten her yeri ilgilendiren işler için global, paylaşılan bir politika için
controller, tek seferlik bir durum için route.

## Filtre olarak yetkilendirme

`#[authorize("ability")]` (ya da `#[authorize(role = "admin")]`), handler'dan önce çalışan bir
filtredir; mevcut kullanıcıyı uygulamanın `Gate`'ine göre kontrol eder ve `401` (giriş yapılmamış)
ya da `403` (reddedildi) response'u verir. [Yetkilendirme](../security/authorization) bölümünde
ele alınır.

```rust
#[post("/posts")]
#[authorize(role = "editor")]
async fn store(/* ... */) -> Result<Redirect> { /* ... */ }
```

## Hata render'ını geçersiz kılmak

Hata pazarlığı katmanı, istisna filtresinin ta kendisidir; çıktısını `App::builder().on_error(...)`
ile değiştirebilirsin. Closure, hatayı ve bazı request bilgilerini alır; işi devralmak istiyorsa
`Some(response)`, varsayılan JSON/HTML davranışına bırakmak istiyorsa `None` döndürür:

```rust
use rustali::http::{response::{IntoResponse, Redirect}, StatusCode};

App::builder().on_error(|error, request| {
    // Tarayıcıları dostça bir 404 sayfasına gönder, API çağıranları JSON varsayılanına bırak.
    (error.status == StatusCode::NOT_FOUND && !request.wants_json)
        .then(|| Redirect::to("/not-found").into_response())
})
```

`error` bir `ErrorBody`'dir (`status`, `title`, isteğe bağlı herkese açık bir `detail` ve bir
`debug` bayrağı); `request` ise bir `ErrorRequest`'tir (`path`, `wants_json`, `request_id`,
`debug`). İstemciye göre farklı davranmak için `wants_json`'ı kullan — bu, varsayılan pazarlığın
da kullandığı sinyalin aynısıdır.


## Yapılandırma gerektiren katmanlar

Bazı katmanlar (session'lar, CSRF) ancak uygulama kurulduktan sonra oluşan yapılandırma
değerlerine ihtiyaç duyar. Bunları, bağlamın hazır olduğu sırada çalışan `configure` içine
eklersin:

```rust
App::builder()
    .provider(SessionProvider)
    .configure(session_layers)   // çerez-oturumu + CSRF katmanlarını ekler
    .controllers()
    .run()
    .await
```

[CSRF & Session'lar](../security/csrf)'a bak.
