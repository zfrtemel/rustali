# Kurulum

## Gereksinimler

- **Rust** 1.97 veya daha yenisi (2024 edition). [rustup](https://rustup.rs) ile kur.
- **Windows'ta**, MSVC araç zinciri için "Desktop development with C++" iş yüküyle birlikte
  *Visual Studio Build Tools* gerekir (`link.exe`'yi o sağlar).
- Başlangıç için veritabanı isteğe bağlıdır — varsayılan **SQLite**'tır ve hiçbir şey kurmayı
  gerektirmez. PostgreSQL ve MySQL de desteklenir.

## CLI ile (önerilen)

```bash
cargo install rustali-cli
rustali new blog
cd blog
cargo run -- migrate    # SQLite şemasını oluştur
cargo run               # http://127.0.0.1:8000 üzerinde sun
```

`rustali new` konvansiyonel bir proje iskeleti oluşturur; ne ürettiğinin turu için
[İlk Uygulaman](first-app) sayfasına bak. View katmanı olmadan yalnızca JSON üreten bir
proje için `--preset api` ekle.

## Mevcut bir crate'e

Rustali'yi ve bir async runtime ekle:

```toml
# Cargo.toml
[dependencies]
rustali = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde = { version = "1", features = ["derive"] }
```

Asgari bir `main.rs`:

```rust
use rustali::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    App::builder().controllers().run().await
}
```

## Özellik bayrakları (feature flags)

`rustali` meta crate'i her alt sistemi bir Cargo özelliğiyle açar. Varsayılanlar tipik bir
full-stack uygulamayı kapsar; daha hafif bir derleme için bunları kırp.

| Özellik | Varsayılan | Getirdiği |
|---|---|---|
| `macros` | ✅ | `#[controller]`, `#[derive(Entity)]`, `#[derive(Validate)]` |
| `orm` | ✅ | ORM, migration'lar, `Db` extractor'ı |
| `sqlite` | ✅ | SQLite driver'ı |
| `postgres` | | PostgreSQL driver'ı |
| `mysql` | | MySQL driver'ı |
| `views` | ✅ | `View<T>`, Askama backend'i |
| `tera` | | Tera (çalışma zamanı) view backend'i |
| `sessions` | ✅ | Çerez session'ları, flash, CSRF |
| `validation` | ✅ | Validation kuralları ve `Validated<T>` |
| `auth` | ✅ | Parola özetleme, guard'lar, `Auth`/`Login` (`sessions`'ı gerektirir) |
| `cache` | ✅ | `Cache` trait'i ve bellek deposu |
| `redis` | | Redis destekli cache deposu |
| `mail` | ✅ | `Mailable` ve taşıyıcılar |
| `queue` | ✅ | Arka plan işleri ve bir worker |
| `test` | | Süreç içi `TestClient` (`dev-dependencies`'e koy) |

Özellikleri kırpmak için `default-features = false` ile varsayılanları kapat ve yalnızca
ihtiyacın olanı listele. Örneğin, view'süz, Postgres üzerinde bir JSON API:

```toml
[dependencies]
rustali = { version = "0.1", default-features = false, features = ["macros", "orm", "postgres", "auth", "validation"] }
```

Kırparken akılda tutulacak birkaç şey:

- Bir veritabanı driver'ı `orm`'den ayrıdır: `orm`'i **ve** `sqlite` / `postgres` / `mysql`
  driver'larından birini tut. `orm`'i driver'sız açmak sana ORM tiplerini verir ama bağlanacak
  bir şey vermez.
- `auth`, `sessions`'ı gerektirir; bu yüzden API ağırlıklı bir derlemede bile çerez session'ları
  ve CSRF elde edersin — gerçekten ikisini de istemiyorsan `auth`'u da çıkar.
- `redis` ve `tera`, `cache` ve `views`'in isteğe bağlı eklentileridir; temel özellik de açık
  olmadıkça hiçbir şey yapmazlar.

`rustali new --preset api` bu daha hafif şekli senin için oluşturur: view katmanı dışarıda
bırakılmış, yalnızca JSON üreten bir proje.

## Editör kurulumu

[rust-analyzer](https://rust-analyzer.github.io/) kur. Kontrol komutunu, framework'ün
kullandığı lint'lerin aynısı için clippy'ye yönlendir:

```json
// .vscode/settings.json
{ "rust-analyzer.check.command": "clippy" }
```

Sıradaki: [İlk Uygulaman](first-app).
