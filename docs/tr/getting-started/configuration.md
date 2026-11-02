# Yapılandırma

Tüm yapılandırma, düz ve okunabilir anahtarlar olarak tek bir **`.env`** dosyasında (ya da
gerçek süreç ortamında) yaşar. Bölüm başına yapılandırma dosyası yoktur — her şey için tek bir
yer.

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

Yani dağıtım platformunun ortamında `PORT`'u ayarlamak, `.env`'deki `PORT`'u geçersiz kılar; o
da yerleşik varsayılanı geçersiz kılar. Bir `.env` dosyası bir geliştirici kolaylığıdır;
üretim değerleri gerçek ortamdan gelir ve kazanır.

## Anahtarlar

Her ayar temiz, düz bir anahtardır. Yaygın olanlar:

| Anahtar | Ayarladığı |
|---|---|
| `APP_NAME` | uygulama adı (loglar, varsayılan sayfa başlıkları) |
| `APP_ENV` | `development` \| `test` \| `production` |
| `APP_DEBUG` | yanıtlarda ayrıntılı hata detayı (üretimde kapat) |
| `APP_KEY` | çerezleri ve JWT'leri imzalayan sır (≥ 16 karakter) |
| `HOST` / `PORT` | bağlanılacak adres (`127.0.0.1` / `8000`) |
| `LOG_LEVEL` / `LOG_JSON` | tracing filtresi; `true` olduğunda satır başına bir JSON nesnesi |
| `DATABASE_URL` | veritabanı bağlantı dizesi |
| `DATABASE_MAX_CONNECTIONS` / `DATABASE_LOGGING` | havuz boyutu; her ifadeyi logla |
| `SOFT_DELETE` | silmek yerine `deleted_at` damgala ([ayrıntılar](../database/base-entity)) |
| `CACHE_DRIVER` / `CACHE_URL` | önbellek arka ucu ([ayrıntılar](../deeper/cache)) |
| `MAIL_TRANSPORT` / `MAIL_FROM` / `MAIL_URL` | e-posta arka ucu ([ayrıntılar](../deeper/mail)) |
| `QUEUE_CAPACITY` | süreç içi kuyruk boyutu ([ayrıntılar](../deeper/queues)) |
| `SESSION_COOKIE` / `SESSION_LIFETIME_MINUTES` / `SESSION_CSRF` | oturumlar ([ayrıntılar](../security/csrf)) |
| `OPENAPI_TITLE` / `OPENAPI_VERSION` | OpenAPI belge bilgisi |

Değerler en belirgin tipe ayrıştırılır — `PORT=8000` bir tamsayıdır, `APP_DEBUG=false` bir
bool'dur — bu yüzden yanlış bir tip (örn. `PORT=abc`) başlangıçta net bir hatadır, sessiz bir
varsayılan değil.

## Ortam başına ayarlar

Profil dosyaları yoktur; ortam başına değerleri her ortamın kendisinde ayarla. Geliştiriciye
uygun bir `.env` commit'le ve üretimde gerçek değerleri (sırlar, `APP_ENV=production`,
`APP_DEBUG=false`) dağıtım platformunun ortamı aracılığıyla ayarla; orada `.env`'i geçersiz
kılarlar.

`APP_DEBUG` üretimde otomatik olarak kapatılmaz — orada `APP_DEBUG=false` ayarla. Debug
açıkken, hata yanıtları gerçek kullanıcılara göstermek istemeyeceğin iç detayları (yığın
mesajları, veritabanı hataları) içerir.

## Anahtar

`APP_KEY`, oturum çerezlerini ve JWT'leri imzalar ve en az 16 karakter olmalıdır. Geliştirmede
iskelet tek kullanımlık bir anahtar getirir; üretimde ortamdan gerçek bir sır ayarla ve asla
commit'leme:

```bash
# dağıtım platformunun ortamı (commit'lenen .env değil)
APP_KEY=a-long-random-secret-at-least-32-characters
```

## Yapılandırmayı okumak

Bir handler içinde, yapılandırmaya bağlam üzerinden eriş:

```rust
async fn home(ctx: Ctx) -> String {
    ctx.config().app.name.clone()
}
```

`ctx.config().is_production()`, `APP_ENV == "production"` için bir kolaylıktır.

## Özel ayarlar

Yerleşik anahtarların ötesindeki paket ve uygulama ayarları, `Config::section` ile tipli
bölümler olarak okunur. Özel bir bölüm için değerlerini `APP__SECTION__FIELD` kaçış yoluyla
ayarla (çift alt çizgi iç içe geçmeyi ayırır):

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

Tip `Default`'u uyguladığında ve değerler yok olabileceğinde `section_or_default::<T>("name")`
kullan.

> Yapılandırmaya ihtiyaç duyan bir paket (veritabanı, önbellek, e-posta, …), bölümünü
> [provider](../deeper/providers)'ı içinde bu şekilde okur; böylece framework ve uygulama
> yapılandırması için tek, tutarlı bir mekanizma olur.
