# Yapılandırma

Tüm yapılandırma, sade ve okunaklı anahtarlar hâlinde tek bir **`.env`** dosyasında (ya da
gerçek süreç ortamında) durur. Her bölüm için ayrı bir yapılandırma dosyası yoktur — her şey tek
bir yerde toplanır.

```bash
# .env
APP_NAME=My App
APP_ENV=development        # development | test | production
APP_DEBUG=true            # yanıtlarda ayrıntılı hata detayı — üretimde kapat
APP_KEY=a-long-dev-secret-change-me

DATABASE_URL=sqlite://db/app.db?mode=rwc
```

## Öncelik

En düşükten en yükseğe, her katman bir öncekini geçersiz kılar:

1. Derlenmiş varsayılanlar (her alanın bir tane var — örn. port `8000`'dir).
2. `.env` dosyası.
3. Gerçek süreç ortam değişkenleri.

Yani `PORT`'u dağıtım platformunun ortamında ayarlarsan bu, `.env`'deki `PORT`'u geçersiz kılar;
`.env`'deki değer de yerleşik varsayılanı geçersiz kılar. `.env` dosyası yalnızca geliştirme
kolaylığıdır; üretim değerleri gerçek ortamdan gelir ve her zaman öncelik kazanır.

## Anahtarlar

Her ayar, sade ve tek düzeyli bir anahtardır. En sık kullanılanlar şunlar:

| Anahtar | Ayarladığı |
|---|---|
| `APP_NAME` | uygulama adı (loglar, varsayılan sayfa başlıkları) |
| `APP_ENV` | `development` \| `test` \| `production` |
| `APP_DEBUG` | response'larda ayrıntılı hata detayı (üretimde kapat) |
| `APP_KEY` | çerezleri ve JWT'leri imzalayan sır (≥ 16 karakter) |
| `HOST` / `PORT` | bağlanılacak adres (`127.0.0.1` / `8000`) |
| `LOG_LEVEL` / `LOG_JSON` | tracing filtresi; `true` olduğunda satır başına bir JSON nesnesi |
| `DATABASE_URL` | veritabanı bağlantı dizesi |
| `DATABASE_MAX_CONNECTIONS` / `DATABASE_LOGGING` | havuz boyutu; her ifadeyi logla |
| `SOFT_DELETE` | silmek yerine `deleted_at` damgala ([ayrıntılar](../database/base-entity)) |
| `CACHE_DRIVER` / `CACHE_URL` | cache backend'i ([ayrıntılar](../deeper/cache)) |
| `MAIL_TRANSPORT` / `MAIL_FROM` / `MAIL_URL` | e-posta backend'i ([ayrıntılar](../deeper/mail)) |
| `QUEUE_CAPACITY` | süreç içi queue boyutu ([ayrıntılar](../deeper/queues)) |
| `SESSION_COOKIE` / `SESSION_LIFETIME_MINUTES` / `SESSION_CSRF` | session'lar ([ayrıntılar](../security/csrf)) |
| `OPENAPI_TITLE` / `OPENAPI_VERSION` | OpenAPI belge bilgisi |

Değerler, en uygun tipe otomatik ayrıştırılır — `PORT=8000` bir tamsayı, `APP_DEBUG=false` bir
bool olur — bu yüzden tipi yanlış bir değer (örneğin `PORT=abc`) başlangıçta açık bir hataya yol
açar; sessizce bir varsayılana düşmez.

## Ortam başına ayarlar

Profil dosyaları yoktur; her ortama özel değerleri doğrudan o ortamın kendisinde ayarla.
Geliştirme için uygun bir `.env` commit'le; üretimdeki gerçek değerleri (sırlar,
`APP_ENV=production`, `APP_DEBUG=false`) ise dağıtım platformunun ortamı üzerinden ver — bu
değerler orada `.env`'i geçersiz kılar.

`APP_DEBUG` üretimde kendiliğinden kapanmaz — orada `APP_DEBUG=false` olarak ayarlaman gerekir.
Debug açıkken hata response'ları, gerçek kullanıcılara göstermek istemeyeceğin iç ayrıntıları
(yığın izleri, veritabanı hataları) içerir.

## Anahtar

`APP_KEY`, session çerezlerini ve JWT'leri imzalar; en az 16 karakter olmalıdır. Geliştirmede
iskelet, atılabilir bir anahtar üretir; üretimde ise ortamdan gerçek bir sır ver ve bunu asla
commit'leme:

```bash
# dağıtım platformunun ortamı (commit'lenen .env değil)
APP_KEY=a-long-random-secret-at-least-32-characters
```

## Yapılandırmayı okumak

Bir handler içinde yapılandırmaya bağlam üzerinden erişebilirsin:

```rust
async fn home(ctx: Ctx) -> String {
    ctx.config().app.name.clone()
}
```

`ctx.config().is_production()`, `APP_ENV == "production"` kontrolü için pratik bir kısayoldur.

## Özel ayarlar

Yerleşik anahtarların dışındaki paket ve uygulama ayarları, `Config::section` ile tipli bölümler
hâlinde okunur. Özel bir bölümün değerlerini `APP__SECTION__FIELD` biçiminde ayarla (çift alt
çizgi, iç içe geçmiş alanları birbirinden ayırır):

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Payments {
    provider: String,
    retries: u32,
}

// .env:  APP__PAYMENTS__PROVIDER=stripe   APP__PAYMENTS__RETRIES=3
let payments: Option<Payments> = ctx.config().section("payments")?;
```

Tip `Default`'u implemente ediyorsa ve değerler hiç tanımlanmamış olabilecekse
`section_or_default::<T>("name")` kullan.

> Yapılandırmaya ihtiyaç duyan bir paket (veritabanı, cache, e-posta, …), kendi bölümünü
> [provider](../deeper/providers)'ı içinde işte bu şekilde okur; böylece framework ve uygulama
> yapılandırması tek ve tutarlı bir mekanizma üzerinden yürür.
