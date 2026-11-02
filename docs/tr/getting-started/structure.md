# Proje Yapısı

`rustali new` küçük, konvansiyonel bir düzen oluşturur. Buradaki hiçbir şey sihir değil —
sıradan Rust'tır ve yeniden adlandırabilir ya da yeniden düzenleyebilirsin — ama konvansiyonu
izlemek, üreteçlerin ve `App::builder().controllers()`'ın kodunu bulmasını sağlar.

## Yepyeni bir proje

```text
blog/
├── Cargo.toml
├── .env                  # tüm yapılandırma (tek dosya)
├── db/
│   └── .gitkeep          # SQLite dosyası, migration'lar ve anlık görüntü burada yaşar
├── src/
│   ├── main.rs           # App::builder()...run()
│   └── controllers.rs    # bir home controller: "/" ve "/api/ping"
└── templates/
    └── home.html         # `api` preset'inde atlanır
```

Her üst düzey parçanın ne işe yaradığı:

- **`.env`** — tek dosyada tüm yapılandırma: `APP_*`, `DATABASE_URL` ve her isteğe bağlı paket
  için bir satır. [Yapılandırma](configuration)'ya bak. Gerçek sırlar üretimde ortamdan gelir
  ve `.env`'i geçersiz kılar.
- **`db/`** — SQLite dosyası (geliştirmede), migration'ların ve `migrate:diff`'i yönlendiren
  şema anlık görüntüsü.
- **`src/`** — kodun. `main.rs` uygulamayı önyükler; gerisi, üreteçlerin oluşturduğu
  controller'lar, modeller vb.'dir.
- **`templates/`** — binary'ye derlenen Askama template'leri (`api` preset'li bir projede yoktur).

## `main.rs`

Giriş noktası bilinçli olarak küçüktür — uygulamanın kullandığı provider'ları adlandırır ve
her controller'ı bağlar:

```rust
mod controllers;

use rustali::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    App::builder()
        .provider(DatabaseProvider)
        .controllers()
        .run()
        .await
}
```

`run()` aynı zamanda konsol giriş noktası olarak da çalışır: argümansız sunar; bir komutla
(`migrate`, `routes`, `queue:work`, ...) bunun yerine onu çalıştırır.
[Konsol Komutları](../deeper/console)'na bak.

## Üretilen kod nereye gider

`rustali make:<şey> <Ad>`, `src/`'nin konvansiyonel alt dizinlerine yazar:

| Komut | Yazdığı yer |
|---|---|
| `make:controller` | `src/controllers/` |
| `make:model` | `src/models/` |
| `make:migration` | `src/migrations/` |
| `make:job` | `src/jobs/` |
| `make:mail` | `src/mail/` |
| `make:middleware` | `src/middleware/` |
| `make:policy` | `src/policies/` |
| `make:test` | `tests/` |

Bir üreteç, `--force` geçmedikçe mevcut bir dosyanın üzerine asla yazmaz.
`make:controller Post`'tan sonra, derlenmesi için modülü bildir (`mod controllers;` içinde
`pub mod post;`); controller ardından `controllers()` ile otomatik bağlanır.

## Tek dosyanın ötesine büyümek

Bir uygulama büyüdükçe, bir lib + bin ayrımı yararlıdır — entegrasyon testlerinin `main`'in
oluşturduğu aynı uygulamayı kurmasını sağlar. `blog` örneği tam olarak bunu yapar:

```rust
// src/lib.rs
pub mod controllers;
pub mod migrations;
pub mod models;

pub fn app() -> AppBuilder {
    App::builder()
        .provider(DatabaseProvider)
        .provider(SessionProvider)
        .migrations::<migrations::Migrator>()
        .controllers()
        .configure(session_layers)
}

// src/main.rs
#[tokio::main]
async fn main() -> Result<()> {
    blog::app().run().await
}
```

Artık bir test `blog::app()`'i çağırabilir ve onu [`TestClient`](../deeper/testing) ile
sürebilir — üretimle aynı provider'lar, route'lar ve middleware.
