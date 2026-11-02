# Middleware & Filtreler

Middleware, bir request'i girişte ve response'u çıkışta sarar. Rustali'de middleware yalnızca bir
[tower] `Layer`'dır ve onu üç düzeyde iliştirirsin — global, controller başına ya da route
başına — böylece enine kesen kaygılar (hız sınırlama, CORS, auth, loglama) handler'larının
dışında kalır. Bu, framework'ün tasarımındaki katmanlı filtre hattıdır.

[tower]: https://docs.rs/tower

## Varsayılan yığın

Her uygulama, en dıştan içe doğru bir varsayılan yığınla sarılır:

1. **Request ID** — bir `x-request-id` (bir UUID) atar ve yayar.
2. **Tracing** — `tracing` aracılığıyla yapılandırılmış request/response loglaması.
3. **Catch-panic** — panikleyen bir handler, bağlantıyı düşürmek yerine bir `500` olur.
4. **Bağlam enjeksiyonu** — düz tower katmanlarının servislere erişebilmesi için `AppContext`'i
   request uzantılarına koyar.
5. **Hata pazarlığı** — bir framework [`Error`](responses)'ını, istemciye bağlı olarak JSON'a ya
   da bir HTML sayfasına çevirir ("istisna filtresi").
6. **Senin katmanların** — aşağıya eklediğin her şey.
7. **Route'lar.**

Bunları asla sen eklemezsin; hep açıktırlar.

## Global middleware

`App::builder().layer(...)` ile her route'un çevresine bir katman ekle:

```rust
use rustali::{middleware::Cors, prelude::*};

App::builder()
    .layer(Cors::permissive())
    .controllers()
    .run()
    .await
```

Global katmanların varsayılan yığının *içinde* durur; bu yüzden request id'yi hâlâ görürler ve
hataları hâlâ tutarlı bir şekilde render edilir.


### CORS

`Cors`, `tower_http`'nin yeniden dışa aktarılmış `CorsLayer`'ıdır. `Cors::permissive()` herhangi
bir kaynağa, metoda ve başlığa izin verir — geliştirmede kullanışlı, ama üretim için genellikle
neye izin verdiğini adlandırmak istersin:

```rust
use rustali::http::{HeaderValue, Method};
use rustali::middleware::Cors;

let cors = Cors::new()
    .allow_origin("https://app.example.com".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST]);

App::builder().layer(cors)
```

CORS'a yalnızca bir tarayıcı API'ni farklı bir kaynaktan çağırdığında ihtiyacın olur;
aynı kaynaklı bir SSR uygulaması buna ihtiyaç duymaz.

## Controller ve route middleware'i

`impl` üzerindeki `#[middleware(expr)]` ile bütün bir controller'a ya da metot üzerindeki
`#[middleware(expr)]` ile tek bir route'a bir katman iliştir. Metot katmanları, controller
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

`expr`, tipi bir tower `Layer` olan herhangi bir değerdir — `Throttle` gibi bir framework
katmanı, `axum::middleware::from_fn(my_fn)` gibi bir axum yardımcısı ya da kendi katmanın.

## Hız sınırlama

`Throttle`, sabit pencereli, bellek içi bir sınırlayıcıdır. Bir `x-ratelimit-remaining`
başlığı ekler ve sınıra ulaşıldığında `Retry-After` ile `429 Too Many Requests` response'u verir.

```rust
Throttle::per_minute(60)             // 60 istek / dakika
Throttle::per_second(10)             // 10 / saniye
Throttle::new(100, Duration::from_secs(300))   // 5 dakikada 100
```

İstemciler `X-Forwarded-For` (ilk adres) ya da eş IP'ye göre anahtarlanır. Bu tek süreçli
uygulamadır; paylaşılan bir depo (Redis) onun arkasına takılacaktır.

## Kendininkini yazmak

Herhangi bir axum middleware fonksiyonu çalışır. Onu `from_fn` ile sar ve uygun olan hangi
düzeyse oraya ekle:

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

`rustali make:middleware Timing` bu şekli senin için oluşturur.

### Kısa devre yapmak

Bir middleware'in `next.run(...)`'ı çağırması gerekmez. Ondan önce bir response döndür, handler
asla çalışmaz — bir gate'in, bir bakım anahtarının ya da bir başlık kontrolünün bir request'i
erkenden reddetme yolu budur:

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

Bir middleware'in yapabileceği iki şey — `next.run`'dan önce request üzerinde hareket etmek ve
sonrasında response üzerinde hareket etmek — tüm yüzeyi kapsar: kimlik doğrulama, zamanlama,
başlık enjekte etme, response'ları yeniden yazma ve bir request'i doğrudan reddetme.

### Düzeyler nasıl iç içe geçer

Üç iliştirme noktası, dıştan içe doğru iç içe geçer: **global → controller → route → handler**.
Dıştaki bir katman request'i ilk, response'u son görür. Yani global bir zamanlama katmanı içindeki her
şeyi ölçer, bir controller `Throttle`'ı bir route'un kendi daha katı `Throttle`'ından önce
sayar ve bir route `#[authorize]`'u handler'a en yakın çalışır. İşi yapan en dar düzeye başvur:
gerçekten enine kesen kaygılar için global, paylaşılan bir politika için controller, bir
seferlik için route.

## Filtre olarak yetkilendirme

`#[authorize("ability")]` (ya da `#[authorize(role = "admin")]`), handler'dan önce çalışan,
mevcut kullanıcıyı uygulamanın `Gate`'ine karşı kontrol eden ve `401` (giriş yapılmamış) ya da
`403` (reddedildi) response'u veren bir filtredir. [Yetkilendirme](../security/authorization)'de
ele alınmıştır.

```rust
#[post("/posts")]
#[authorize(role = "editor")]
async fn store(/* ... */) -> Result<Redirect> { /* ... */ }
```

## Hata render'ını geçersiz kılmak

Hata pazarlığı katmanı istisna filtresidir; çıktısını `App::builder().on_error(...)` ile
değiştir. Closure, hatayı ve bazı request bilgilerini alır ve devralmak için `Some(response)` ya
da varsayılan JSON/HTML'e geri dönmek için `None` döndürür:

```rust
use rustali::http::{response::{IntoResponse, Redirect}, StatusCode};

App::builder().on_error(|error, request| {
    // Tarayıcıları dostça bir 404 sayfasına gönder, API çağıranları JSON varsayılanına bırak.
    (error.status == StatusCode::NOT_FOUND && !request.wants_json)
        .then(|| Redirect::to("/not-found").into_response())
})
```

`error` bir `ErrorBody`'dir (`status`, `title`, isteğe bağlı bir herkese açık `detail` ve bir
`debug` bayrağı); `request` bir `ErrorRequest`'tir (`path`, `wants_json`, `request_id`,
`debug`). İstemciye göre dallanmak için `wants_json`'ı kullan — varsayılan pazarlığın kullandığı
sinyalin aynısı.


## Yapılandırma gerektiren katmanlar

Bazı katmanlar (session'lar, CSRF) yalnızca uygulama kurulduktan sonra var olan yapılandırma
değerlerine ihtiyaç duyar. Bunları, bağlam mevcutken çalışan `configure` içine ekle:

```rust
App::builder()
    .provider(SessionProvider)
    .configure(session_layers)   // çerez-oturumu + CSRF katmanlarını ekler
    .controllers()
    .run()
    .await
```

[CSRF & Session'lar](../security/csrf)'a bak.
