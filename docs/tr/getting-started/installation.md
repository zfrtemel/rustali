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

`rustali new <ad>` sana iki soru sorar — bir **proje tipi (preset)** ve isteğe bağlı bir
**paket** kümesi — ve ardından projeyi iskeletler. Hiç bayrak vermezsen seni etkileşimli olarak
yönlendirir; bayraklar ise script veya CI için bu soruları atlamanı sağlar. Yeni bir projenin
neler içerdiğini gezmek için [İlk Uygulaman](first-app) sayfasına bak.

### Proje tipi (preset)

| Preset | Sana verdiği | Notlar |
|---|---|---|
| `api` | Swagger'lı JSON API; `openapi` açık, view yok. Controller metin bir `/` ve JSON bir `/api/ping` sunar. | `.routes(openapi_router())` bağlar. `openapi` paketini önceden işaretler. |
| `fullstack` | SSR view'lar + session'lar + `auth`; `templates/home.html` ile Askama view'ları, ayrıca kendi view'leriyle çalışan bir register/login/logout akışı. | `auth` varsayılan olarak açık → users modeli + `create_users` migration'ı + session bağlantıları + `AuthController`. `auth` paketini önceden işaretler. |
| `minimal` | Çıplak çekirdek: ORM ve tek bir controller. | View yok, önceden işaretli ek paket yok. |

`--preset` vermezsen `rustali new`, yukarıdaki üç preset'i sunan etkileşimli bir **Project type**
seçicisi gösterir. `--api`, `--preset api`'nin kısa yazılışıdır ve ayrıca paket sorusunu da
atlar (`--no-prompt` vermiş gibi).

### Paketler

Bir preset'in üzerine isteğe bağlı paketler ekleyebilirsin. Ne `--with` ne `--no-prompt` (ne de
`--api`) verirsen, `rustali new`, preset'in ima ettiği paket önceden işaretli olacak şekilde
etkileşimli bir **Extra packages** çoklu seçim listesi gösterir.

| Paket | Eklediği | `.env` |
|---|---|---|
| `cache` | Bellek içi cache deposu + provider | `CACHE_DRIVER=memory` |
| `mail` | Mail taşıyıcısı + provider | `MAIL_TRANSPORT=log`, `MAIL_FROM=no-reply@example.com` |
| `queue` | Arka plan queue'su + worker | `QUEUE_CAPACITY=1024` |
| `auth` | Session'lar + auth; `src/models/user.rs`, `create_users` migration'ı, `.migrations::<Migrator>()`, `.configure(session_layers)` ve çalışan bir auth controller — view'li preset'te SSR register/login/logout + view'ler, aksi halde JWT bearer token'lı JSON API (`/api/register`, `/api/login`, `/api/me`) | `SESSION_CSRF=true` |
| `openapi` | OpenAPI/Swagger router'ı | `OPENAPI_TITLE=API` |

Her paket, `rustali` bağımlılığında bir Cargo feature'ı açar, `main.rs`'e bir `.provider(...)`
ya da `.routes(...)` satırı ekler ve kendi `.env` satır(lar)ını yazar. `auth` ek olarak user
modelini/migration'ını, session bağlantılarını ve çalışmaya hazır bir `src/controllers/auth.rs`
üretir (view'li preset'te SSR + view'ler, view yoksa JSON + JWT). Bir preset, ima ettiği paketi
önceden işaretler (`api` → `openapi`, `fullstack` → `auth`).

### Etkileşimli ve bayraklar

- **Etkileşimli** (ilgili bayrak yoksa): önce bir **Project type** `Select`'i, sonra preset'in
  paketi önceden işaretli bir **Extra packages** `MultiSelect`'i.
- **Etkileşimsiz** (script/CI): tipi seçmek için `--preset <api|fullstack|minimal>`, paketleri
  seçmek için `--with pkg1,pkg2` ver ve/veya kalan soruyu atlamak için `--no-prompt` kullan.
  `--api`, `--no-prompt`'u ima eder.

```bash
rustali new blog                                 # etkileşimli: önce proje tipi, sonra paketler
rustali new api --preset api                     # api preset'i; ek paketleri yine de sorar
rustali new api --api                            # aynı preset, ama paket sorusunu atlar
rustali new shop --preset fullstack              # SSR view'lar + session'lar + auth
rustali new svc --preset minimal --with cache,queue  # çıplak çekirdek, artı cache ve queue
rustali new svc --preset minimal --no-prompt     # çıplak çekirdek, hiç soru yok
rustali new blog --force                         # mevcut bir dizinin üzerine yaz
```

### Her preset ne üretir

Preset ne olursa olsun her proje aynı tabanla gelir: bir `.provider(DatabaseProvider)`, bir
`controllers.rs`, tek bir `.env` (`config/*.toml` değil) ve `DATABASE_URL=sqlite://db/app.db?mode=rwc`
ile SQLite.

Bu tabanın üzerine:

- **`api`**, `openapi` paketini ekler: `.routes(openapi_router())` ile bağlanan bir Swagger
  router'ı ve metin bir `/` ile JSON bir `/api/ping` sunan bir controller. View katmanı yok.
- **`fullstack`**, `auth` paketini ekler: `templates/home.html` ile Askama view'ları, bir users
  modeli ve `create_users` migration'ı, ayrıca session bağlantıları.
- **`minimal`**, tabanın ötesine hiçbir şey eklemez — yalnızca ORM ve tek bir controller.

Üretilen `main.rs`, sabit bir sırayla kurulur: `DatabaseProvider` → `auth` varsa
(`SessionProvider`, `AuthProvider`) → cache/mail/queue provider'ları → `auth` varsa
`.migrations::<Migrator>()` → `.controllers()` → `openapi` varsa `.routes(openapi_router())` →
`auth` varsa `.configure(session_layers)` → `.run().await`.

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

`api` preset'i, bu daha hafif biçimi senin yerine kurar: view katmanı dışarıda bırakılmış,
yalnızca JSON döndüren bir proje. Yukarıdaki [Proje tipi (preset)](#proje-tipi-preset) bölümüne
bak.

## Editör kurulumu

[rust-analyzer](https://rust-analyzer.github.io/) kur. Framework'le aynı lint'leri görmek için
kontrol komutunu clippy'ye yönlendir:

```json
// .vscode/settings.json
{ "rust-analyzer.check.command": "clippy" }
```

Sıradaki: [İlk Uygulaman](first-app).
