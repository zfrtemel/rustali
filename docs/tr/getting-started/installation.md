# Kurulum

## Gereksinimler

- **Rust** 1.97 veya daha yenisi (2024 edition). [rustup](https://rustup.rs) ile kur.
- **Windows'ta** MSVC araç zinciri için *Visual Studio Build Tools* gerekir; kurarken
  "Desktop development with C++" iş yükünü de seç (`link.exe` bu iş yükünden gelir).
- Başlarken veritabanı zorunlu değildir — varsayılan **SQLite**'tır ve ayrıca hiçbir şey
  kurmana gerek kalmaz. İstersen PostgreSQL ve MySQL de desteklenir.

## CLI ile (önerilen)

```bash
cargo install rustali-cli
rustali new blog
cd blog
cargo run -- migrate    # SQLite şemasını oluştur
cargo run               # http://127.0.0.1:8000 üzerinde sun
```

`rustali new`, konvansiyona uygun bir proje iskeleti oluşturur; neler ürettiğini gezmek için
[İlk Uygulaman](first-app) sayfasına bak. View katmanı olmadan yalnızca JSON döndüren bir
proje istiyorsan `--preset api` ekle.

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

`rustali` meta crate'i, her alt sistemi ayrı bir Cargo özelliğiyle açar. Varsayılanlar tipik
bir full-stack uygulamanın ihtiyacını karşılar; daha hafif bir derleme istiyorsan bunları kırp.

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

Özellikleri kırpmak için `default-features = false` diyerek varsayılanları kapat ve yalnızca
ihtiyacın olanları tek tek listele. Örneğin, view'süz, Postgres üzerinde çalışan bir JSON API
şöyle görünür:

```toml
[dependencies]
rustali = { version = "0.1", default-features = false, features = ["macros", "orm", "postgres", "auth", "validation"] }
```

Kırparken birkaç noktayı aklında tut:

- Veritabanı driver'ı `orm`'den ayrı bir özelliktir: hem `orm`'i, hem de `sqlite` / `postgres`
  / `mysql` driver'larından birini açık tut. `orm`'i driver olmadan açarsan ORM tiplerini
  alırsın ama bağlanacak bir veritabanın olmaz.
- `auth`, `sessions`'a bağımlıdır; bu yüzden API ağırlıklı bir derlemede bile çerez
  session'ları ve CSRF devreye girer — ikisini de gerçekten istemiyorsan `auth`'u da kapat.
- `redis` ve `tera`, sırasıyla `cache` ve `views` için isteğe bağlı eklentilerdir; temel özellik
  açık olmadıkça tek başlarına bir işe yaramazlar.

`rustali new --preset api`, bu daha hafif biçimi senin yerine kurar: view katmanı dışarıda
bırakılmış, yalnızca JSON döndüren bir proje.

## Editör kurulumu

[rust-analyzer](https://rust-analyzer.github.io/) kur. Framework'le aynı lint'leri görmek için
kontrol komutunu clippy'ye yönlendir:

```json
// .vscode/settings.json
{ "rust-analyzer.check.command": "clippy" }
```

Sıradaki: [İlk Uygulaman](first-app).
