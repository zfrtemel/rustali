# İlk Uygulaman

```bash
cargo install rustali-cli     # 0.1.0 yayımlandığında
rustali new blog
cd blog
cargo run -- migrate          # SQLite şemasını oluştur
cargo run                     # http://127.0.0.1:8000
```

HTML ana sayfasını görmek için `http://127.0.0.1:8000` adresini aç; iskelette ayrıca hemen
deneyebileceğin bir JSON route'u da hazır gelir:

```bash
curl http://127.0.0.1:8000/api/ping
# {"ok":true}
```

Bu küçük proje bile temel fikri ortaya koyuyor: aynı uygulama, tek bir controller üzerinden
hem bir HTML sayfası hem de bir JSON API sunabiliyor.

## Elde ettiklerin

`rustali new`, konvansiyona uygun bir proje iskeleti oluşturur:

```text
blog/
├── Cargo.toml
├── .env                  # tüm yapılandırma: name, env, debug, key, DATABASE_URL, …
├── db/                   # SQLite dosyası + migration'lar + şema anlık görüntüsü
├── src/
│   ├── main.rs           # App::builder()...run()  (≈ 8 satır)
│   └── controllers.rs    # bir home controller: SSR "/" ve JSON "/api/ping"
└── templates/
    └── home.html
```

Uygulamanın tüm önyükleme süreci `src/main.rs` içindedir — bir builder, veritabanı provider'ı
ve otomatik bağlanan controller'lar:

```rust
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

`src/controllers.rs` ise yukarıda bağlanan home controller'ıdır — biri HTML, biri JSON olan iki
route'un nasıl yan yana durduğuna dikkat et:

```rust
#[controller("/")]
impl HomeController {
    #[get("/", name = "home")]
    async fn home() -> &'static str {
        "Welcome to Rustali!"
    }

    #[get("/api/ping", name = "ping")]
    async fn ping() -> Json<Ping> {
        Json(Ping { ok: true })
    }
}
```

## Bir kaynak ekle

```bash
rustali make:model Post          # src/models/post.rs  (#[derive(Entity)])
rustali make:controller Post     # src/controllers/post.rs
rustali make:migration create_posts
```

Tablonu tanımlamak için modeldeki kolonları düzenle:

```rust
// src/models/post.rs
use rustali::{orm::chrono::{DateTime, Utc}, prelude::*};
use serde::Serialize;

#[derive(Entity, Debug, Clone, Serialize)]
#[table("posts")]
pub struct Post {
    pub id: i64,
    #[column(max_len = 200)]
    pub title: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Ardından yaptığın bu değişiklikten migration'ı üret — diff, entity'ni okuyup ilk taslağı senin
için yazar:

```bash
cargo run -- migrate:diff "create posts"   # bir migration dosyası + anlık görüntü yazar (sen gözden geçirirsin)
cargo run -- migrate
```

::: tip
`migrate:diff`, ürettiği migration'ı `db/migrations/` altına yazar. Derlemeye dahil olması için
onu `src/migrations/` içine taşı ve `Migrator`'da listele — bu adımın otomatikleşmesi yol
haritasında var. Sürecin tamamı için [Migration'lar](../database/migrations) sayfasına bak.
:::

Controller'ı bağla (`App::builder().controllers()` zaten binary'deki her `#[controller]`'ı
otomatik toplar); artık CRUD'un hazır. Sıradaki: [Routing & Controller](../basics/routing).
