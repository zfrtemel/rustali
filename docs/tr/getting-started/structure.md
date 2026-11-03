# Proje Yapısı

`rustali new`, küçük ve konvansiyona uygun bir düzen oluşturur. Burada sihirli hiçbir şey yok —
hepsi sıradan Rust'tır; dosyaları yeniden adlandırabilir ya da yeniden düzenleyebilirsin — ama
konvansiyona uyarsan üreteçler ve `App::builder().controllers()` kodunu kendiliğinden bulur.

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
    └── home.html         # yalnızca fullstack preset'inde
```

`templates/` ve `home.html` yalnızca `fullstack` preset'inde üretilir; `src/models.rs` ve
`src/migrations.rs` ise yalnızca `auth` paketi seçildiğinde (ki `fullstack` bunu önceden
işaretler) ortaya çıkar.

Her üst düzey parçanın görevi şöyle:

- **`.env`** — tüm yapılandırma tek dosyada: `APP_*`, `DATABASE_URL` ve her isteğe bağlı paket
  için birer satır. [Yapılandırma](configuration) sayfasına bak. Üretimde gerçek sırlar ortamdan
  gelir ve `.env`'deki değerleri geçersiz kılar.
- **`db/`** — SQLite dosyası (geliştirme ortamında), migration'lar ve `migrate:diff`'e kaynaklık
  eden şema anlık görüntüsü.
- **`src/`** — kodun. `main.rs` uygulamayı önyükler; geri kalanı ise üreteçlerin oluşturduğu
  controller'lar, modeller ve benzeridir.
- **`templates/`** — binary'ye derlenen Askama template'leri (yalnızca `fullstack`
  preset'inde bulunur).

## `main.rs`

Giriş noktası bilinçli olarak küçük tutulmuştur — yalnızca uygulamanın kullandığı provider'ları
sayar ve controller'ların hepsini bağlar:

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

`run()` aynı zamanda konsol giriş noktası olarak da iş görür: argümansız çağrıldığında
uygulamayı sunar; bir komut verirsen (`migrate`, `routes`, `queue:work`, ...) sunmak yerine o
komutu çalıştırır. [Konsol Komutları](../deeper/console) sayfasına bak.

## Üretilen kod nereye gider

`rustali make:<şey> <Ad>`, dosyaları `src/` altındaki konvansiyonel alt dizinlere yazar:

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

Bir üreteç, `--force` vermedikçe mevcut bir dosyanın üzerine asla yazmaz.
`make:controller Post`'tan sonra, dosyanın derlemeye girmesi için modülü bildir (`mod controllers;`
içine `pub mod post;` ekle); controller bundan sonra `controllers()` ile otomatik bağlanır.

## Tek dosyanın ötesine büyümek

Uygulama büyüdükçe lib + bin ayrımına geçmek işe yarar — böylece entegrasyon testleri, `main`'in
kurduğu uygulamanın aynısını kurabilir. `blog` örneği tam olarak bunu yapar:

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

Artık bir test `blog::app()`'i çağırıp onu [`TestClient`](../deeper/testing) üzerinden
çalıştırabilir — üretimdekiyle birebir aynı provider'lar, route'lar ve middleware ile.
