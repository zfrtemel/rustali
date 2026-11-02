# İlk Uygulaman

```bash
cargo install rustali-cli     # 0.1.0 yayımlandığında
rustali new blog
cd blog
cargo run -- migrate          # SQLite şemasını oluştur
cargo run                     # http://127.0.0.1:8000
```

HTML ana sayfası için `http://127.0.0.1:8000` adresini aç; iskelet ayrıca hemen
kullanabileceğin bir JSON route'u da sunar:

```bash
curl http://127.0.0.1:8000/api/ping
# {"ok":true}
```

Bu tek proje bile ana fikri gösteriyor — aynı uygulama tek bir controller'dan hem bir HTML
sayfası hem de bir JSON API sunuyor.

## Elde ettiklerin

`rustali new` konvansiyonel bir proje iskeleti oluşturur:

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

`src/main.rs` tüm önyükleme sürecidir — bir builder, veritabanı provider'ı ve otomatik
bağlanan controller'lar:

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

Ve `src/controllers.rs`, bağladığı home controller'ıdır — biri HTML biri JSON olan iki
route'un yan yana olduğuna dikkat et:

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

Tablonu tanımlamak için modelin kolonlarını düzenle:

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

Ardından değişiklikten migration'ı üret — diff, entity'ni okur ve ilk taslağı yazar:

```bash
cargo run -- migrate:diff "create posts"   # gözden geçirilmiş bir migration + anlık görüntü yazar
cargo run -- migrate
```

::: tip
`migrate:diff`, üretilen migration'ı `db/migrations/` altına yazar. Derlenmesi için onu
`src/migrations/` içine taşı ve `Migrator`'ında listele — otomatik yerleştirme yol
haritasında. Tam workflow için [Migration'lar](../database/migrations) sayfasına bak.
:::

Controller'ı bağla (`App::builder().controllers()` zaten binary'deki her `#[controller]`'ı
toplar) ve CRUD'un hazır. Sıradaki: [Routing & Controller](../basics/routing).
