# Rustali — Rust Web Framework Vizyon & Mimari Dokümanı

> **Proje adı (geçici, 2026-10-31):** Rustali — crate/CLI `rustali`, alt crate'ler
> `rustali-*`. İleride değiştirilebilir; uygulama planı
> `C:\Users\zafer\.claude\plans\fable-ile-bu-plan-synthetic-dragonfly.md`.

> **Kimin için yazıldı:** İkili amaç — (1) senin kişisel yol haritan ve öğrenme planın,
> (2) ileride katkıcılara/topluluğa sunabileceğin bir vizyon belgesi taslağı.
> Çalışma dili Türkçe; ileride topluluğa açılırsa İngilizce'ye çevrilmeli.

**Tarih:** 2026-10-31
**Durum:** Taslak v0.1 — tartışmaya ve değişime açık

---

## 0. Önce Dürüst Gerçek (Oku, geçme)

Bu bölüm dokümanın en önemli kısmı. Gerisi bunun üzerine kurulu.

1. **"Laravel'i Rust'a portlamak" işe yaramaz.** Laravel'in büyüsü PHP'nin
   dinamikliğinden gelir: Facade'lar, `__call` magic metodları, runtime service
   container binding'i, Eloquent'in ActiveRecord sihri. Rust statik tipli,
   ownership temelli bir dildir. Birebir port, Rust'ı kötü bir PHP'ye çevirir.

2. **Rust'ta zaten olgun parçalar var.** Axum (HTTP), SeaORM/sqlx (ORM),
   Tera/Askama (template), apalis (queue), validator (doğrulama). Senin işin
   bunları **yeniden yazmak değil**, Laravel ruhuyla **birleştirmek**.

3. **Loco.rs bu alanı büyük ölçüde doldurdu.** "Rails/Laravel for Rust" iddiasıyla
   çıktı, 1.0'a ulaştı, SeaORM 2.0 + Axum üzerine kurulu, batteries-included:
   models, controllers, jobs, mailers, auth, CLI, test desteği. Sıfırdan kopyası
   anlamsız olur.

4. **Sen Rust'a yenisin.** Tek başına Laravel ölçeğinde framework yazıp topluluk
   kurmak gerçekçi değil — bunu caydırmak için değil, enerjini doğru yere koyman
   için yazıyorum. Laravel'in arkasında 13 yıl ve yüzlerce katkıcı var.

**Bu yüzden asıl hedef yeniden tanımlanıyor:**

> **Framework yazarak Rust'ta ustalaşmak.** Çıktı (framework) ikincil; birincil
> kazanç, bu yolculukta öğreneceklerin. Bu, Rust öğrenmenin en güçlü
> yöntemlerinden biridir.

---

## 1. Scope: Full-Stack (SSR) + API — Tek Çatı, İki Mod

**Karar (2026-10-31):** Framework hem **full-stack (SSR)** hem **API** olacak —
tıpkı Laravel ve .NET MVC gibi. Aynı uygulama hem hazır HTML sayfa döndürebilir
hem JSON API verebilir.

```rust
#[controller]
impl UserController {
    #[get("/users")]                              // FULL-STACK (SSR)
    async fn index() -> Html { render("users.html", data) }

    #[get("/api/users")]                          // API
    async fn api_index() -> Json<Vec<User>> { ... }
}
```

- **SSR tarafı:** Sunucu HTML üretir (Askama/Tera — Ayrışma #4). İnteraktiflik
  gerekirse **htmx / Alpine.js** gibi hafif JS (ağır frontend framework YOK).
- **API tarafı:** Aynı controller'lar JSON döndürür; mobil/SPA istemcileri besler.
- **KAPSAM DIŞI (bilinçli):** Rust WASM frontend (Leptos/Dioxus/Yew). Bu tek
  başına Laravel kadar büyük bir iştir; Rust'a yeni biri için projeyi
  bitirilemez yapar. İleride *opsiyonel entegrasyon* olarak düşünülebilir, çekirdek
  hedef değil.

**Mimariye etkisi (önemli):** "Tek çatı, iki mod" bazı kararları belirler:
- **Exception filter (Ayrışma #3) content-negotiation yapmalı:** API isteğine
  JSON hata, SSR isteğine HTML hata sayfası döndürür (`Accept` başlığına göre).
- **Response tipi esnek olmalı:** handler `Html` ya da `Json<T>` döndürebilsin;
  ortak bir `IntoResponse` soyutlaması (Rust'ta zaten var).
- **Auth iki modda çalışmalı:** SSR için cookie/session, API için token/JWT.

---

## 2. Konumlandırma: Nereye Oynuyoruz?

Loco zaten var olduğuna göre üç seçenek var:

| Seçenek | Açıklama | Tavsiye |
|---|---|---|
| **A. Loco'ya katkı** | Mevcut projeye katkıcı ol, eksiklerini kapat | Topluluk için en hızlı etki, ama "senin projen" olmaz |
| **B. Niş farklılaşma** | Loco'nun zayıf olduğu bir alana odaklan | **Önerilen** — öğrenme + özgünlük dengesi |
| **C. Sıfırdan klon** | Her şeyi baştan yaz | Öğrenmek için iyi, topluluk için anlamsız |

### Önerilen: B + C karışımı (öğrenme için C, konumlandırma için B)

Öğrenirken sıfırdan parçalar yaz (C'nin eğitici değeri), ama projenin kimliğini
Loco'dan **bir farklılaşma ekseni** üzerine kur. Olası farklılaşma eksenleri:

- **Laravel + .NET Core sentezi (ÖNE ÇIKAN ADAY)** — Laravel'in DX'i + .NET'in
  güçlü fikirleri. İki somut ayrışma noktası: **attribute routing** (route'u
  controller'ın üstünde tanımla, ayrı route dosyası yok) ve **code-first diff
  migration** (EF Core tarzı). Bkz. Bölüm 4.5.
- **Laravel-benzeri DX'e aşırı sadakat** — Loco Rails'e benziyor; sen özellikle
  PHP/Laravel'den gelen geliştiricilere hitap edebilirsin (migration isimleri,
  `artisan`-vari CLI, Blade-vari şablon hissi).
- **Gömülü/edge-first** — tek binary, SQLite-first, sıfır-config deployment.
- **Admin panel / CRUD generator** — Laravel Nova / Filament benzeri, Rust'ta eksik.
- **Türkçe-first dokümantasyon** — küçük ama gerçek bir topluluk boşluğu.

> **Alınan karar (2026-10-31):** Framework kör kopya değil, **bilinçli sentez**
> olacak. Laravel mükemmel değil; .NET Core'un daha iyi çözdüğü yerleri alıyoruz.
> İki ana ayrışma: attribute routing + code-first migration. Detay Bölüm 4.5.

> **Karar bekliyor:** Hangi eksende farklılaşacağız? (Doküman sonundaki "Açık
> Sorular" bölümüne bak.)

---

## 3. Teknoloji Temeli (Yeniden yazmayacaklarımız)

Bunlar kanıtlanmış, olgun crate'ler. Üzerlerine kuruyoruz:

| Katman | Seçim | Neden |
|---|---|---|
| Async runtime | **Tokio** | Fiili standart |
| HTTP/Routing | **Axum** | Tokio ekibinden, sağlıklı, tower middleware |
| ORM | **SeaORM** (motor) + bizim `#[derive(Entity)]` katmanı | SeaORM açık/modüler; üstüne .NET POCO hissi + code-first diff örülecek. Bkz. Ayrışma #2a/#2b |
| Migration | SeaORM migrator / sqlx-migrate | |
| Template/View | **Askama** (varsayılan, tip-güvenli=Razor) + **Tera** (opsiyon, runtime=Blade) | Bkz. Ayrışma #4 |
| Validation | `validator` crate | |
| Serialization | `serde` | Fiili standart |
| CLI | `clap` | `artisan` benzeri komutlar için |
| Config | `figment` / `config` | env + dosya katmanlı config |
| Queue/Jobs | `apalis` + Redis | |
| Logging | `tracing` | |

**Kural:** Bir crate olgunsa, onu sarmala (wrap), yeniden yazma. Yeniden yazmak
sadece *öğrenmek* için, ayrı bir egzersiz branch'inde yapılır.

---

## 4. Mimari Taslağı (Laravel kavramları → Rust karşılıkları)

| Laravel | Bu framework'te | Rust'ta nasıl |
|---|---|---|
| Service Container | AppContext / state struct | Axum `State<T>` + dependency struct |
| Facade (`DB::`, `Cache::`) | Modül fonksiyonları | Facade YOK — açık (explicit) API. "No magic" |
| Route (`Route::get`) | Router builder | `axum::Router` sarmalayıcısı |
| Controller | Handler fonksiyonları | `async fn(...) -> Result<Response>` |
| Middleware | Tower layer | `tower::Layer` / Axum middleware |
| Eloquent Model | Entity + repository | SeaORM Entity |
| Migration | Migration dosyaları | SeaORM migrator |
| Blade | Template (Tera) | Tera şablonları |
| Request validation | Extractor + validator | Custom extractor + `validator` |
| Artisan | CLI (`clap`) | `cargo run -- <komut>` veya ayrı binary |
| Queue/Job | Worker + job trait | `apalis` |
| Env config | Config katmanı | `figment` |

> **Kritik tasarım kararı:** Laravel'in "magic"ini KOPYALAMA. Rust'ta magic yerine
> **explicit + tip güvenliği** satılır. Loco bile "No macros, no magic" diyor.
> Convention-over-configuration'ı dosya yapısı ve CLI generator'larla sağla,
> runtime magic'le değil.

---

## 4.5. Laravel + .NET Core Sentezi (Framework'ün Kimliği)

Laravel mükemmel değil. .NET Core'un bazı fikirleri daha iyi. Kör kopya yerine
en iyi parçaları birleştiriyoruz. Bu bölüm framework'ün **ayrışma eksenini** tarif
eder — yani "neden Loco yerine bunu kullanayım?" sorusunun cevabı.

### .NET Core'dan alınacak fikirler

| Konu | Laravel (zayıf) | .NET Core (iyi) | Bizim hedefimiz |
|---|---|---|---|
| Route tanımlama | Ayrı route dosyası, handler'ı ayrıca bağla | **Attribute routing** — route handler'ın üstünde | Attribute routing (makro) |
| Migration | Manuel yazılır | **Code-first diff** (EF Core) | Code-first diff (ileri hedef) |
| Dependency Injection | Runtime magic container | Açık, compile-time'a yakın DI | Açık DI — Rust'a zaten uygun |
| Tip güvenliği | Zayıf (PHP) | Güçlü | Rust'ta zaten var |

### Ayrışma #1 — Attribute Routing (route tanımlama amelesini bitir)

**Dert:** Axum ve Laravel'de çift iş var — hem handler'ı yaz, hem ayrı yerde
route'a bağla.

```rust
// Axum'da ŞU AN (çift iş, amele):
async fn index() -> ... {}
async fn show() -> ... {}
let app = Router::new()
    .route("/users", get(index))        // ayrıca bağla
    .route("/users/:id", get(show));    // ayrıca bağla
```

**Hedeflenen his (.NET attribute routing gibi):**

```rust
#[controller("/users")]
impl UserController {
    #[get("/")]
    async fn index() -> Result<Json<Vec<User>>> { ... }

    #[get("/:id")]
    async fn show(Path(id): Path<i32>) -> Result<Json<User>> { ... }
}
// Makro route'ları otomatik toplar — ayrı Router::new() YOK
```

- **Teknik:** Rust attribute makrosu (proc-macro). Actix-web ve Rocket'ta zaten var
  (`#[get("...")]`), Axum'da yok. Axum üstüne bu katmanı biz ekleriz.
- **Dezavantaj (dürüstlük):** Route'ları tek dosyada toplu göremezsin (Laravel'in
  sevilen `route:list`'i bunu çözer). **Önlem:** `app routes` CLI komutu tüm
  route'ları makrolardan toplayıp listeler — dezavantajı kapatır.
- **Öğrenme maliyeti:** proc-macro yazmak Rust'ta ileri konu. Faz 1'de ÖNCE açık
  router ile başla, attribute routing'i Faz 4 civarı makro öğrenince ekle.

### ORM seçimi — SeaORM (motor), üstüne bizim katman

**Kriterler:** (1) model/entity tanımı, (2) üstüne kendi katmanımızı ekleyebilmek,
(3) code-first diff. Rust adayları:

| ORM | Model hissi | Üstüne katman eklenir mi | Code-first diff |
|---|---|---|---|
| **SeaORM** ✅ | ActiveRecord'a yakın | **Evet — modüler, açık ("no hidden magic")** | Kısmi; diff'i biz yazarız |
| Diesel | Schema-first, katı | Zor — makro ağır, iç yapı kapalı | Hayır |
| sqlx | ORM değil, ham SQL | En açık ama her şeyi sen yazarsın | Hayır |

**Karar:** **SeaORM'i motor olarak kullan.** Açık ve modüler olduğu için üstüne
kendi API'mizi ve code-first diff katmanımızı örebiliriz. Diesel buna kapalı,
sqlx'te entity bile yok.

> **Dürüstlük:** Hiçbir Rust ORM'i EF Core'un `add-migration` diff'ini tam
> sağlamaz. SeaORM db↔entity üretir ama entity→migration diff'ini **biz
> yazacağız.** İşte "üstüne geliştireceğimiz" asıl özgün değer bu.

### Ayrışma #2a — Model/Entity Yapısı (.NET POCO hissi)

**Tercih (2026-10-31):** .NET Core'un (EF Core) **POCO** model yapısı tercih
ediliyor — sade, okunur, tek sınıf. SeaORM'in varsayılan yapısı bundan uzak:
`Model` + `ActiveModel` + `Entity` + `Relation` enum ayrı ayrı, makro ağır.

**.NET POCO (sevilen):**
```csharp
public class User {
    public int Id { get; set; }
    public string Name { get; set; }
    public List<Post> Posts { get; set; }   // ilişki sade bir property
}
```

**SeaORM varsayılanı (gürültülü):**
```rust
#[derive(DeriveEntityModel)] #[sea_orm(table_name="users")]
pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub name: String }
#[derive(DeriveRelation)] pub enum Relation { /* ilişkiler ayrı enum */ }
```

**Hedefimiz — POCO sadeliği + Rust tip güvenliği (SeaORM üstüne bizim makro):**
```rust
#[derive(Entity)]
#[table("users")]
struct User {
    #[key]
    id: i32,
    name: String,
    email: String,

    #[has_many]
    posts: Vec<Post>,   // ilişki sade bir alan — .NET gibi
}
```

- **Teknik:** `#[derive(Entity)]` proc-makromuz, tek sade struct'ı alır, altta
  SeaORM'in gürültülü yapısına (Model/ActiveModel/Relation) **otomatik çevirir.**
  Kullanıcı POCO yazar, SeaORM'i hiç görmez.
- **Bu, "üstüne geliştireceğimiz ORM" dileğinin tam karşılığı:** motor SeaORM,
  yüz bizim.
- **Zorluk:** attribute routing ile birlikte framework'ün en zor iki proc-macro
  parçasından biri. **Faz 4+ işi.** Başlangıçta ham SeaORM entity'siyle çalış,
  sade API'yi sonra giydir.

### Ayrışma #2b — Code-First Diff Migration (EF Core tarzı)

**Dert:** Laravel'de her şema değişikliğinde migration'ı elle yazarsın.

**Hedeflenen his:**
```rust
// 1. Entity'yi değiştir (yukarıdaki POCO struct'a alan ekle):
struct User { id, name, email, phone /* YENİ alan */ }

// 2. Komut:
$ app migrate diff
// → framework önceki şema snapshot'ı ile farkı hesaplar,
//   'add phone column' migration'ını OTOMATİK üretir
```

- **Teknik:** Framework bir **şema snapshot'ı** tutmalı, entity'lerle karşılaştırıp
  diff üretmeli. Bu, EF Core'un en karmaşık parçalarından biridir.
- **Gerçeklik:** SeaORM bunu tam sağlamaz. Bu katmanı kendimiz yazarız —
  **projenin en zor ama en özgün parçası.**
- **Tavsiye:** İleri hedef (Faz 3 sonrası). Önce Laravel-tarzı manuel-ama-generator'lı
  migration ile başla, diff'i sonra ekle. Diff yarım kalırsa bile framework çalışır.

### Ayrışma #3 — Katmanlı Filter Pipeline (.NET'in en güçlü fikri)

**Dert:** Laravel middleware tek boyutludur. .NET ise isteği katman katman,
**farklı seviyelerde** ve **deklaratif** şekilde işler. Yetki, hata, loglama gibi
kesişen dertleri (cross-cutting concerns) kodu kirletmeden bağlarsın.

**.NET'in katmanları → bizim hedefimiz:**

| .NET kavramı | Ne yapar | Seviye | Rust karşılığı |
|---|---|---|---|
| Global middleware | Tüm isteklere (logging, CORS) | Uygulama | tower `Layer` (app geneli) |
| Controller middleware | O controller'ın route'larına | Controller | `#[middleware(...)]` attribute |
| Action/method filter | Tek endpoint'e | Metot | metot üstü attribute |
| **Authorization filter** | Controller'a girmeden yetki kontrolü | Her seviye | `#[authorize(...)]` |
| **Exception filter** | Hatayı yakalar, standart cevaba çevirir | Her seviye | merkezi error + `IntoResponse` |
| Action filter | Metot öncesi/sonrası (before/after) | Metot | before/after hook |

**Hedeflenen his — katmanlı, deklaratif:**

```rust
#[controller("/admin")]
#[authorize(role = "admin")]        // controller bazlı yetki filtresi
#[middleware(RateLimit)]            // controller bazlı middleware
impl AdminController {

    #[get("/users")]
    async fn list() -> ... { ... }   // admin yetkisini miras alır

    #[delete("/users/:id")]
    #[authorize(permission = "user.delete")]  // metot bazlı EK yetki
    async fn delete() -> ... { ... }
}
```

- **Teknik — iyi haber:** Axum zaten **tower `Layer`/`Service`** soyutlaması
  üstünde duruyor. Yani katmanlı pipeline mimarisi HAZIR; biz sadece ona
  deklaratif (attribute) bir yüz takıyoruz. Bu fikir Rust'a zorlama değil,
  tam oturuyor.
- **Exception filter:** Rust'ta merkezi `Error` tipi + `IntoResponse` ile doğal
  karşılığı var (Bölüm, Faz 2). .NET'teki gibi hatayı yakalayıp tutarlı JSON
  cevaba çevirir.
- **Authorization filter:** Axum extractor + `#[authorize]` makrosu. İstek
  handler'a girmeden yetki reddi.
- **Miras (inheritance) kuralı:** Controller seviyesindeki filtreler metotlara
  miras olur; metot kendi filtresini ekleyebilir (.NET davranışı).
- **Öğrenme maliyeti:** tower `Layer`/`Service` Rust'ta orta-ileri konu. Önce
  (Faz 1) basit global middleware ile başla; controller/metot bazlı deklaratif
  filtreler makro öğrenince (Faz 4+) gelir.

### Ayrışma #4 — View / Template Katmanı (Blade vs Razor felsefesi)

**Dert:** View katmanı framework'ün tam bir parçası. Ama Blade (Laravel) ve Razor
(.NET) iki zıt felsefeyi temsil eder ve senin sentez hedefin burada bir **seçim**
dayatıyor.

| Konu | Blade (Laravel) | Razor (.NET) |
|---|---|---|
| Tip güvenliği | **Yok** — değişken yoksa sayfa açılınca patlar | **Var** — `@model User`, derleme anında kontrol |
| Derleme | Runtime | Derleme zamanında sınıfa |
| Hata ne zaman | Çalışma anında (prod'da) | **Derleme anında** (`build` patlar) |
| His | Rahat, esnek, hot-reload kolay | Disiplinli, güvenli |

**Rust'taki üç aday (= üç felsefe):**

| Crate | Felsefe | Benzediği | Artı / Eksi |
|---|---|---|---|
| **Tera** | Runtime, Jinja/Blade sözdizimi | **Blade** | + Hot-reload kolay, tasarımcı dostu / − Hata çalışma anında |
| **Askama** | Derleme-zamanı, Jinja sözdizimi, `.rs`'e derlenir | **Razor** | + Tip-güvenli, hata `build`'de / − Hot-reload zor, her değişiklik recompile |
| **Maud** | Rust makrosuyla HTML (template dosyası yok) | — (kendine özgü) | + Tam tip-güvenli, en hızlı / − HTML'i Rust içinde yazarsın, tasarımcı dostu değil |

**Sentez felsefesiyle çatışma:**
- Senin hedefin "Laravel DX + .NET disiplini". Ama view'de ikisi **aynı anda** olmaz:
  - Razor disiplini istiyorsan → **Askama** (tip-güvenli, Rust ruhuna uygun, ama Blade'in rahatlığı yok).
  - Blade rahatlığı istiyorsan → **Tera** (esnek, hot-reload, ama tip güvenliği yok — bu Rust'ın "derleme anında yakala" ruhuna ters).

**Önerilen çözüm — ikisini de sun, varsayılanı Askama yap:**
> Framework **her ikisini** destekleyen bir view trait'i tanımlasın:
> `trait View { fn render(&self) -> Result<Html>; }`. Varsayılan olarak **Askama**
> (tip-güvenli, Rust ruhu, .NET disiplini). İsteyen `Tera` backend'ine geçebilsin
> (prototipleme ve tasarımcı dostu). Böylece sentez korunur: disiplin varsayılan,
> rahatlık opsiyon.

- **Full-stack sorusuyla bağlantı:** Eğer ileride SSR/reaktif frontend (Leptos
  tarzı) düşünülüyorsa bu tamamen ayrı bir dünya. Şimdilik **server-rendered
  HTML** (Blade/Razor dünyası) hedefleniyor; reaktif frontend Bölüm 7 "açık soru".
- **Öğrenme maliyeti:** Tera en kolay başlangıç (makro yok). Askama makro/derleme
  bilgisi ister ama Rust'ın en öğretici parçalarından. Faz 6'da **önce Tera ile
  çalıştır**, Askama'yı tip-güvenlik dersi olarak sonra ekle.

### Özet: Felsefe

> Laravel'in **DX'ini** (açınca çalışır, generator'lar, konvansiyon) + .NET'in
> **mühendislik disiplinini** (attribute routing, code-first migration, katmanlı
> filter pipeline, tip-güvenli view, açık DI, tip güvenliği) birleştir. "No magic" ilkesine sadık
> kal: makro kod üretir ama ürettiği kod görünür/öngörülebilir olmalı — runtime
> sihir değil.
>
> **Dört ayrışma ekseni:** (1) attribute routing, (2) code-first diff migration,
> (3) katmanlı deklaratif filter pipeline, (4) tip-güvenli view (Askama varsayılan,
> Tera opsiyon). "Neden Loco yerine bunu?" sorusunun cevabı bu dörttür.

---

## 5. Aşamalı Yol Haritası (Öğrenme + İnşa Birlikte)

Rust'a yeni olduğun için her aşama hem bir öğrenme hedefi hem bir ürün parçası.

### Faz 0 — Rust Temelleri (framework'ten ÖNCE)
- [ ] Rust Book'u bitir (ownership, borrowing, lifetimes, traits, error handling)
- [ ] `Result`/`Option`, `?` operatörü, custom error (thiserror/anyhow)
- [ ] async/await temel kavramı, Tokio "hello world"
- **Çıktı:** Küçük bir CLI aracı (ör. todo list) — framework değil, kas hafızası.

### Faz 1 — Minimal HTTP Katmanı
- [ ] Axum ile çıplak bir "hello world" server
- [ ] Router sarmalayıcı: `app.get("/", handler)` hissi
- [ ] Basit middleware (logging) ekle
- **Öğrenme:** trait, generics, Axum extractor sistemi
- **Çıktı:** `framework-core` crate'inin ilk hali

### Faz 2 — Yapılandırma + Hata Yönetimi
- [ ] Katmanlı config (env + toml)
- [ ] Merkezi hata tipi ve HTTP'ye dönüşümü (IntoResponse)
- **Çıktı:** Her uygulamanın tutarlı hata ve config deneyimi

### Faz 3 — Veritabanı Katmanı
- [ ] SeaORM entegrasyonu, AppContext'e DB havuzu koy
- [ ] İlk migration + basit CRUD
- [ ] Repository pattern (Eloquent hissi ama explicit)
- **Çıktı:** DB'li çalışan örnek uygulama

### Faz 4 — CLI Generator'lar (Laravel'in gizli silahı)
- [ ] `clap` ile komut iskeleti
- [ ] `generate controller/model/migration` komutları (dosya şablonu üretir)
- **Öğrenme:** Bu, DX'in kalbi. Convention'ı burada enforce edersin.
- **Çıktı:** `new`, `generate`, `migrate`, `serve` komutları

### Faz 4.5 — Attribute Routing + Filter Pipeline (ayrışma eksenleri)
- [ ] proc-macro öğren, `#[controller]` / `#[get]` makrolarını yaz (Ayrışma #1)
- [ ] `app routes` komutu — tüm route'ları listele (attribute'un dezavantajını kapat)
- [ ] Katmanlı filter: global → controller → metot (Ayrışma #3, tower Layer üstüne)
- [ ] `#[authorize]` ve `#[middleware]` attribute'ları
- **Öğrenme:** proc-macro + tower Service/Layer — ileri Rust. Zorlanırsan Faz 1'in
  açık router'ıyla devam et, bu fazı ertele; framework yine çalışır.
- **Çıktı:** .NET-tarzı deklaratif controller + katmanlı filtre

### Faz 5 — Auth + Session
- [ ] Parola hashleme (argon2), JWT veya cookie session
- [ ] Auth middleware/extractor + `#[authorize]` filtresini gerçek yetkiye bağla
- [ ] Exception filter: merkezi error → tutarlı JSON cevap (Ayrışma #3)
- **Çıktı:** Login/register olan örnek SaaS iskeleti

### Faz 6 — View / Template Katmanı (Ayrışma #4)
- [ ] Önce **Tera** entegrasyonu (runtime, kolay başlangıç), layout/partial sistemi
- [ ] `View` trait'i tanımla — backend değiştirilebilir olsun
- [ ] **Askama** backend'i ekle (tip-güvenli, Razor hissi) ve varsayılan yap
- [ ] `generate view` komutu (CLI generator)
- [ ] **İki mod:** aynı controller hem `Html` (SSR) hem `Json<T>` (API) döndürebilsin
- [ ] Exception filter'ı content-negotiation'a bağla: API→JSON hata, SSR→HTML hata (Bölüm 1)
- **Öğrenme:** Askama = derleme-zamanı template, Rust'ın tip güvenliği dersi
- **Çıktı:** Server-rendered sayfalar + JSON API, tek çatı iki mod (Bölüm 1)

### Faz 7 — Arka Plan İşleri + Zamanlanmış Görevler
- [ ] apalis entegrasyonu, job trait
- [ ] cron-benzeri scheduler
- **Çıktı:** Queue'lu tam yığın iskelet

### Faz 8 — Paketleme + Topluluk
- [ ] `cargo install` ile kurulan CLI
- [ ] Starter template repo
- [ ] Dokümantasyon sitesi (önce Türkçe)
- [ ] İlk `0.1.0` yayını (crates.io)

> **Gerçekçi zaman:** Faz 0-3 bile aylar alabilir. Acele etme; her faz bir
> blog yazısı/öğrenme kaydı olabilir — bu, topluluk inşasının da tohumu.

---

## 6. İsimlendirme & Kimlik

- [ ] Proje adı (Loco, Rocket, Axum gibi kısa ve akılda kalıcı)
- [ ] crates.io'da isim müsaitliği kontrol edilmeli
- [ ] CLI komut adı (`artisan` esprisi: tek kelime, yazması kolay)
- [ ] Logo / slogan (sonraya bırakılabilir)

---

## 7. Riskler ve Tuzaklar

| Risk | Önlem |
|---|---|
| "Her kütüphaneyi portlama" hevesi | Olgun crate'leri sarmala, yeniden yazma |
| Laravel magic'ini zorla taklit | Explicit + tip güvenliği sat; magic yok |
| Rust'a yeniyken aşırı iddialı scope | Faz 0'ı atlama; küçük başla |
| Loco ile birebir rekabet | Farklılaşma eksenine sadık kal |
| Tek kişi tükenmişliği | Her fazı yayınla, geri bildirim al, motivasyon topla |
| Borrow checker ile savaş | Erken aşamada `clone()` kullan, sonra optimize et |
| **proc-macro'ya erken dalma** | Attribute routing/filter makro ister; Rust'a yeniyken tuzak. Önce açık API ile çalıştır, makroyu Faz 4.5'te ekle |
| **code-first diff'in zorluğu** | EF Core'un en zor parçası; yarım kalsa framework yine çalışsın diye manuel migration'ı önce kur |

---

## 8. Açık Sorular (Senin karar vermen gerekenler)

**KARAR VERİLDİ:**
- ✅ **Farklılaşma ekseni:** Laravel DX + .NET Core mühendisliği sentezi.
  Beş eksen: attribute routing, .NET POCO model, code-first diff migration,
  katmanlı filter pipeline, tip-güvenli view. (Bölüm 4.5)
- ✅ **Controller felsefesi:** Attribute routing — route handler'ın üstünde,
  ayrı route dosyası yok. "Route tanımlama amelesi" bitecek.
- ✅ **Migration modeli:** .NET EF Core tarzı code-first diff (ileri hedef).
- ✅ **ORM:** SeaORM motor olarak; üstüne bizim `#[derive(Entity)]` katmanı. (Ayrışma #2a)
- ✅ **Model yapısı:** .NET Core POCO hissi — sade tek struct, `#[derive(Entity)]` ile. (Ayrışma #2a)
- ✅ **View:** Askama varsayılan (Razor/tip-güvenli) + Tera opsiyon (Blade/runtime). (Ayrışma #4)
- ✅ **Scope:** Full-stack (SSR) + API, tek çatı iki mod. WASM frontend kapsam dışı. (Bölüm 1)

**HÂLÂ AÇIK:**
1. **Önce Loco'yu bizzat denedik mi?** Eksiğini görmeden ayrıntılı tasarım erken olabilir.
2. **Proje adı?**

---

## 9. Hemen Sonraki Adım

Doküman onaylandıktan sonra:
1. **Faz 0'a başla** (Rust Book + küçük CLI) — framework kodu YAZMA henüz.
2. Paralelde **Loco.rs'i bizzat çalıştır**, bir örnek uygulama yap, nasıl
   hissettirdiğini not al. Farklılaşma kararı bu deneyimden çıkmalı.
3. Açık soruları netleştir, bu dokümanı v0.2'ye güncelle.

---

## Kaynaklar

- Loco.rs — https://loco.rs
- Rust Web Framework Comparison — https://github.com/flosse/rust-web-framework-comparison
- Best Rust Web Frameworks (Encore) — https://encore.dev/articles/best-rust-web-frameworks
- The Current State of Rust Web Frameworks (LogRocket) — https://blog.logrocket.com/the-current-state-of-rust-web-frameworks/
- Axum — https://github.com/tokio-rs/axum
- SeaORM — https://www.sea-ql.org/SeaORM/
