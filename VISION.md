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

1. **Dinamik dillerin "magic"ini birebir taşımak işe yaramaz.** Facade'lar, runtime
   service container binding'i, ActiveRecord sihri dinamik dillerin doğasından gelir.
   Rust statik tipli, ownership temelli bir dildir; birebir taklit, Rust'ı kötü bir
   dinamik dile çevirir. Rust'ta magic yerine **explicit + tip güvenliği** satılır.

2. **Rust'ta zaten olgun parçalar var.** Axum (HTTP), SeaORM/sqlx (ORM),
   Tera/Askama (template), apalis (queue), validator (doğrulama). Senin işin
   bunları **yeniden yazmak değil**, tutarlı bir çatı altında **birleştirmek**.

3. **Bu alanda başka tam-donanımlı Rust framework'leri var.** Sıfırdan kopyaları
   anlamsız; değer, net bir **farklılaşma ekseninde** (Bölüm 3).

4. **Sen Rust'a yenisin.** Tek başına büyük ölçekli framework yazıp topluluk kurmak
   zordur — bunu caydırmak için değil, enerjini doğru yere koyman için yazıyorum.

**Bu yüzden asıl hedef yeniden tanımlanıyor:**

> **Framework yazarak Rust'ta ustalaşmak.** Çıktı (framework) ikincil; birincil
> kazanç, bu yolculukta öğreneceklerin. Bu, Rust öğrenmenin en güçlü
> yöntemlerinden biridir.

---

## 1. Scope: Full-Stack (SSR) + API — Tek Çatı, İki Mod

**Karar (2026-10-31):** Framework hem **full-stack (SSR)** hem **API** olacak.
Aynı uygulama hem hazır HTML sayfa döndürebilir hem JSON API verebilir.

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
  başına çok büyük bir iştir; Rust'a yeni biri için projeyi bitirilemez yapar.
  İleride *opsiyonel entegrasyon* olarak düşünülebilir, çekirdek hedef değil.

**Mimariye etkisi (önemli):** "Tek çatı, iki mod" bazı kararları belirler:
- **Exception filter (Ayrışma #3) content-negotiation yapmalı:** API isteğine
  JSON hata, SSR isteğine HTML hata sayfası döndürür (`Accept` başlığına göre).
- **Response tipi esnek olmalı:** handler `Html` ya da `Json<T>` döndürebilsin;
  ortak bir `IntoResponse` soyutlaması (Rust'ta zaten var).
- **Auth iki modda çalışmalı:** SSR için cookie/session, API için token/JWT.

---

## 2. Konumlandırma: Nereye Oynuyoruz?

Projenin kimliği net bir **farklılaşma ekseni** üzerine kurulur. Öğrenirken sıfırdan
parçalar yazmanın eğitici değeri korunur, ama ürün olarak öne çıkan şey farklılaşmadır.
Olası eksenler:

- **DX + mühendislik disiplini sentezi (ÖNE ÇIKAN ADAY)** — "açınca çalışır"
  geliştirici deneyimi + sağlam, tipli mühendislik. İki somut ayrışma noktası:
  **attribute routing** (route'u controller'ın üstünde tanımla, ayrı route dosyası yok)
  ve **code-first diff migration**. Bkz. Bölüm 4.5.
- **Gömülü/edge-first** — tek binary, SQLite-first, sıfır-config deployment.
- **Admin panel / CRUD generator** — Rust'ta eksik olan bir alan.
- **Türkçe-first dokümantasyon** — küçük ama gerçek bir topluluk boşluğu.

> **Alınan karar (2026-10-31):** Framework kör kopya değil, **bilinçli sentez**
> olacak: güçlü bir geliştirici deneyimi + tipli mühendislik disiplini. İki ana
> ayrışma: attribute routing + code-first migration. Detay Bölüm 4.5.

---

## 3. Teknoloji Temeli (Yeniden yazmayacaklarımız)

Bunlar kanıtlanmış, olgun crate'ler. Üzerlerine kuruyoruz:

| Katman | Seçim | Neden |
|---|---|---|
| Async runtime | **Tokio** | Fiili standart |
| HTTP/Routing | **Axum** | Tokio ekibinden, sağlıklı, tower middleware |
| ORM | **SeaORM** (motor) + bizim `#[derive(Entity)]` katmanı | SeaORM açık/modüler; üstüne sade tek-struct entity + code-first diff örülecek. Bkz. Ayrışma #2a/#2b |
| Migration | SeaORM migrator / sqlx-migrate | |
| Template/View | **Askama** (varsayılan, derleme-zamanı tip-güvenli) + **Tera** (opsiyon, runtime) | Bkz. Ayrışma #4 |
| Validation | `validator` crate | |
| Serialization | `serde` | Fiili standart |
| CLI | `clap` | Konsol komutları için |
| Config | `figment` | Tek `.env` dosyasından yüklenir (Bkz. ADR-0003) |
| Queue/Jobs | `apalis` + Redis | |
| Logging | `tracing` | |

**Kural:** Bir crate olgunsa, onu sarmala (wrap), yeniden yazma. Yeniden yazmak
sadece *öğrenmek* için, ayrı bir egzersiz branch'inde yapılır.

---

## 4. Mimari Taslağı (Kavram → Rust karşılığı)

| Kavram | Bu framework'te | Rust'ta nasıl |
|---|---|---|
| Servis konteyneri | AppContext / state struct | Axum `State<T>` + dependency struct |
| Global erişimciler | Modül fonksiyonları | Yok — açık (explicit) API. "No magic" |
| Route tanımı | Router builder | `axum::Router` sarmalayıcısı |
| Controller | Handler fonksiyonları | `async fn(...) -> Result<Response>` |
| Middleware | Tower layer | `tower::Layer` / Axum middleware |
| Model | Entity + repository | SeaORM Entity |
| Migration | Migration dosyaları | SeaORM migrator |
| Şablon | Template | Askama / Tera |
| İstek doğrulama | Extractor + validator | Custom extractor + `validator` |
| Konsol komutu | CLI (`clap`) | `cargo run -- <komut>` veya ayrı binary |
| Queue/Job | Worker + job trait | `apalis` |
| Yapılandırma | Config katmanı | `figment` (tek `.env`) |

> **Kritik tasarım kararı:** Dinamik dillerin "magic"ini KOPYALAMA. Rust'ta magic yerine
> **explicit + tip güvenliği** satılır. Convention-over-configuration'ı dosya yapısı ve
> CLI generator'larla sağla, runtime magic'le değil.

---

## 4.5. Framework'ün Kimliği

Kör kopya yerine, güçlü bir geliştirici deneyimini tipli mühendislik disipliniyle
birleştiriyoruz. Bu bölüm framework'ün **ayrışma eksenini** tarif eder — yani "neden bu
framework?" sorusunun cevabı.

### Benimsenen mühendislik fikirleri

| Konu | Bizim hedefimiz |
|---|---|
| Route tanımlama | **Attribute routing** — route handler'ın üstünde (makro) |
| Migration | **Code-first diff** (ileri hedef) |
| Dependency Injection | Açık, compile-time'a yakın DI — Rust'a zaten uygun |
| Tip güvenliği | Rust'ta zaten var |

### Ayrışma #1 — Attribute Routing (route tanımlama amelesini bitir)

**Dert:** Çıplak Axum'da çift iş var — hem handler'ı yaz, hem ayrı yerde
route'a bağla.

```rust
// Axum'da ŞU AN (çift iş, amele):
async fn index() -> ... {}
async fn show() -> ... {}
let app = Router::new()
    .route("/users", get(index))        // ayrıca bağla
    .route("/users/:id", get(show));    // ayrıca bağla
```

**Hedeflenen his (route handler'ın üstünde):**

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
- **Dezavantaj (dürüstlük):** Route'ları tek dosyada toplu göremezsin. **Önlem:**
  `app routes` CLI komutu tüm route'ları makrolardan toplayıp listeler — dezavantajı
  kapatır.
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

> **Dürüstlük:** Hiçbir Rust ORM'i tam otomatik code-first migration diff'ini
> sağlamaz. SeaORM db↔entity üretir ama entity→migration diff'ini **biz
> yazacağız.** İşte "üstüne geliştireceğimiz" asıl özgün değer bu.

### Ayrışma #2a — Model/Entity Yapısı (sade tek-struct hissi)

**Tercih (2026-10-31):** Sade, okunur, **tek struct** model yapısı tercih
ediliyor — ilişkiler bile düz bir alan olarak görünür. SeaORM'in varsayılan
yapısı bundan uzak: `Model` + `ActiveModel` + `Entity` + `Relation` enum ayrı
ayrı, makro ağır.

**SeaORM varsayılanı (gürültülü):**
```rust
#[derive(DeriveEntityModel)] #[sea_orm(table_name="users")]
pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub name: String }
#[derive(DeriveRelation)] pub enum Relation { /* ilişkiler ayrı enum */ }
```

**Hedefimiz — tek-struct sadeliği + Rust tip güvenliği (SeaORM üstüne bizim makro):**
```rust
#[entity]
#[table("users")]
struct User {
    #[key]
    id: i32,
    name: String,
    email: String,

    #[has_many]
    posts: Vec<Post>,   // ilişki sade bir alan — düz bir property gibi
}
```

- **Teknik:** `#[entity]` (kısayol; `#[derive(Entity, Debug, Clone, Serialize)]`'e genişler)
  proc-makromuz, tek sade struct'ı alır, altta
  SeaORM'in gürültülü yapısına (Model/ActiveModel/Relation) **otomatik çevirir.**
  Kullanıcı sade struct'ı yazar, SeaORM'i hiç görmez.
- **Bu, "üstüne geliştireceğimiz ORM" dileğinin tam karşılığı:** motor SeaORM,
  yüz bizim.
- **Zorluk:** attribute routing ile birlikte framework'ün en zor iki proc-macro
  parçasından biri. **Faz 4+ işi.** Başlangıçta ham SeaORM entity'siyle çalış,
  sade API'yi sonra giydir.

### Ayrışma #2b — Code-First Diff Migration

**Dert:** Şema değişikliğinde migration'ı elle yazmak hataya açık ve yorucu.

**Hedeflenen his:**
```rust
// 1. Entity'yi değiştir (yukarıdaki düz struct'a alan ekle):
struct User { id, name, email, phone /* YENİ alan */ }

// 2. Komut:
$ app migrate diff
// → framework önceki şema snapshot'ı ile farkı hesaplar,
//   'add phone column' migration'ını OTOMATİK üretir
```

- **Teknik:** Framework bir **şema snapshot'ı** tutmalı, entity'lerle karşılaştırıp
  diff üretmeli. Bu, code-first migration'ın en karmaşık parçasıdır.
- **Gerçeklik:** SeaORM bunu tam sağlamaz. Bu katmanı kendimiz yazarız —
  **projenin en zor ama en özgün parçası.**
- **Tavsiye:** İleri hedef (Faz 3 sonrası). Önce manuel-ama-generator'lı
  migration ile başla, diff'i sonra ekle. Diff yarım kalırsa bile framework çalışır.

### Ayrışma #3 — Katmanlı Filter Pipeline

**Dert:** Tek boyutlu bir middleware zinciri yetmez. İsteği katman katman,
**farklı seviyelerde** ve **deklaratif** şekilde işlemek gerekir. Yetki, hata,
loglama gibi kesişen dertleri (cross-cutting concerns) kodu kirletmeden bağlarsın.

**Hedeflediğimiz katmanlar:**

| Katman | Ne yapar | Seviye | Rust karşılığı |
|---|---|---|---|
| Global middleware | Tüm isteklere (logging, CORS) | Uygulama | tower `Layer` (app geneli) |
| Controller middleware | O controller'ın route'larına | Controller | `#[middleware(...)]` attribute |
| Metot filtresi | Tek endpoint'e | Metot | metot üstü attribute |
| **Yetki filtresi** | Controller'a girmeden yetki kontrolü | Her seviye | `#[authorize(...)]` |
| **Hata filtresi** | Hatayı yakalar, standart cevaba çevirir | Her seviye | merkezi error + `IntoResponse` |
| Before/after filtresi | Metot öncesi/sonrası | Metot | before/after hook |

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
- **Hata filtresi:** Rust'ta merkezi `Error` tipi + `IntoResponse` ile doğal
  karşılığı var (Bölüm, Faz 2). Hatayı yakalayıp tutarlı JSON cevaba çevirir.
- **Yetki filtresi:** Axum extractor + `#[authorize]` makrosu. İstek
  handler'a girmeden yetki reddi.
- **Miras (inheritance) kuralı:** Controller seviyesindeki filtreler metotlara
  miras olur; metot kendi filtresini ekleyebilir.
- **Öğrenme maliyeti:** tower `Layer`/`Service` Rust'ta orta-ileri konu. Önce
  (Faz 1) basit global middleware ile başla; controller/metot bazlı deklaratif
  filtreler makro öğrenince (Faz 4+) gelir.

### Ayrışma #4 — View / Template Katmanı (derleme-zamanı vs runtime felsefesi)

**Dert:** View katmanı framework'ün tam bir parçası. Ama derleme-zamanı tip-güvenli
şablonlar ile runtime şablonlar iki zıt felsefeyi temsil eder ve bu bir **seçim**
dayatıyor.

| Konu | Runtime şablon | Derleme-zamanı tip-güvenli şablon |
|---|---|---|
| Tip güvenliği | **Yok** — değişken yoksa sayfa açılınca patlar | **Var** — model tipine göre derleme anında kontrol |
| Derleme | Runtime | Derleme zamanında koda |
| Hata ne zaman | Çalışma anında (prod'da) | **Derleme anında** (`build` patlar) |
| His | Rahat, esnek, hot-reload kolay | Disiplinli, güvenli |

**Rust'taki üç aday (= üç felsefe):**

| Crate | Felsefe | Artı / Eksi |
|---|---|---|
| **Tera** | Runtime, Jinja sözdizimi | + Hot-reload kolay, tasarımcı dostu / − Hata çalışma anında |
| **Askama** | Derleme-zamanı, Jinja sözdizimi, `.rs`'e derlenir | + Tip-güvenli, hata `build`'de / − Hot-reload zor, her değişiklik recompile |
| **Maud** | Rust makrosuyla HTML (template dosyası yok) | + Tam tip-güvenli, en hızlı / − HTML'i Rust içinde yazarsın, tasarımcı dostu değil |

**İki felsefe aynı anda olmaz:**
- Derleme-zamanı disiplini istiyorsan → **Askama** (tip-güvenli, Rust ruhuna uygun, ama runtime şablonun rahatlığı yok).
- Runtime rahatlığı istiyorsan → **Tera** (esnek, hot-reload, ama tip güvenliği yok — bu Rust'ın "derleme anında yakala" ruhuna ters).

**Önerilen çözüm — ikisini de sun, varsayılanı Askama yap:**
> Framework **her ikisini** destekleyen bir view trait'i tanımlasın:
> `trait View { fn render(&self) -> Result<Html>; }`. Varsayılan olarak **Askama**
> (tip-güvenli, Rust ruhu, mühendislik disiplini). İsteyen `Tera` backend'ine
> geçebilsin (prototipleme ve tasarımcı dostu). Böylece denge korunur: disiplin
> varsayılan, rahatlık opsiyon.

- **Full-stack sorusuyla bağlantı:** Eğer ileride SSR/reaktif frontend (Leptos
  tarzı) düşünülüyorsa bu tamamen ayrı bir dünya. Şimdilik **server-rendered
  HTML** hedefleniyor; reaktif frontend Bölüm 7 "açık soru".
- **Öğrenme maliyeti:** Tera en kolay başlangıç (makro yok). Askama makro/derleme
  bilgisi ister ama Rust'ın en öğretici parçalarından. Faz 6'da **önce Tera ile
  çalıştır**, Askama'yı tip-güvenlik dersi olarak sonra ekle.

### Özet: Felsefe

> Güçlü bir **geliştirici deneyimini** (açınca çalışır, generator'lar, konvansiyon)
> + sağlam **mühendislik disiplinini** (attribute routing, code-first migration,
> katmanlı filter pipeline, tip-güvenli view, açık DI, tip güvenliği) birleştir.
> "No magic" ilkesine sadık kal: makro kod üretir ama ürettiği kod
> görünür/öngörülebilir olmalı — runtime sihir değil.
>
> **Dört ayrışma ekseni:** (1) attribute routing, (2) code-first diff migration,
> (3) katmanlı deklaratif filter pipeline, (4) tip-güvenli view (Askama varsayılan,
> Tera opsiyon). "Neden bu framework?" sorusunun cevabı bu dörttür.

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
- [ ] Repository pattern (akıcı ama explicit sorgu API'si)
- **Çıktı:** DB'li çalışan örnek uygulama

### Faz 4 — CLI Generator'lar (DX'in gizli silahı)
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
- **Çıktı:** deklaratif controller + katmanlı filtre

### Faz 5 — Auth + Session
- [ ] Parola hashleme (argon2), JWT veya cookie session
- [ ] Auth middleware/extractor + `#[authorize]` filtresini gerçek yetkiye bağla
- [ ] Exception filter: merkezi error → tutarlı JSON cevap (Ayrışma #3)
- **Çıktı:** Login/register olan örnek SaaS iskeleti

### Faz 6 — View / Template Katmanı (Ayrışma #4)
- [ ] Önce **Tera** entegrasyonu (runtime, kolay başlangıç), layout/partial sistemi
- [ ] `View` trait'i tanımla — backend değiştirilebilir olsun
- [ ] **Askama** backend'i ekle (tip-güvenli, derleme-zamanı) ve varsayılan yap
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

- [ ] Proje adı (kısa ve akılda kalıcı)
- [ ] crates.io'da isim müsaitliği kontrol edilmeli
- [ ] CLI komut adı (tek kelime, yazması kolay)
- [ ] Logo / slogan (sonraya bırakılabilir)

---

## 7. Riskler ve Tuzaklar

| Risk | Önlem |
|---|---|
| "Her kütüphaneyi portlama" hevesi | Olgun crate'leri sarmala, yeniden yazma |
| Dinamik dil magic'ini zorla taklit | Explicit + tip güvenliği sat; magic yok |
| Rust'a yeniyken aşırı iddialı scope | Faz 0'ı atlama; küçük başla |
| Mevcut framework'lerle birebir rekabet | Farklılaşma eksenine sadık kal |
| Tek kişi tükenmişliği | Her fazı yayınla, geri bildirim al, motivasyon topla |
| Borrow checker ile savaş | Erken aşamada `clone()` kullan, sonra optimize et |
| **proc-macro'ya erken dalma** | Attribute routing/filter makro ister; Rust'a yeniyken tuzak. Önce açık API ile çalıştır, makroyu Faz 4.5'te ekle |
| **code-first diff'in zorluğu** | code-first migration'ın en zor parçası; yarım kalsa framework yine çalışsın diye manuel migration'ı önce kur |

---

## 8. Açık Sorular (Senin karar vermen gerekenler)

**KARAR VERİLDİ:**
- ✅ **Farklılaşma ekseni:** güçlü geliştirici deneyimi + tipli mühendislik
  disiplini. Beş eksen: attribute routing, sade tek-struct model, code-first
  diff migration, katmanlı filter pipeline, tip-güvenli view. (Bölüm 4.5)
- ✅ **Controller felsefesi:** Attribute routing — route handler'ın üstünde,
  ayrı route dosyası yok. "Route tanımlama amelesi" bitecek.
- ✅ **Migration modeli:** code-first diff (ileri hedef).
- ✅ **ORM:** SeaORM motor olarak; üstüne bizim `#[derive(Entity)]` katmanı. (Ayrışma #2a)
- ✅ **Model yapısı:** sade tek struct hissi, `#[derive(Entity)]` ile. (Ayrışma #2a)
- ✅ **View:** Askama varsayılan (derleme-zamanı/tip-güvenli) + Tera opsiyon (runtime). (Ayrışma #4)
- ✅ **Scope:** Full-stack (SSR) + API, tek çatı iki mod. WASM frontend kapsam dışı. (Bölüm 1)

**HÂLÂ AÇIK:**
1. **Proje adı?**

---

## 9. Hemen Sonraki Adım

Doküman onaylandıktan sonra:
1. **Faz 0'a başla** (Rust Book + küçük CLI) — framework kodu YAZMA henüz.
2. Açık soruları netleştir, bu dokümanı v0.2'ye güncelle.

---

## Kaynaklar

- Rust Web Framework Comparison — https://github.com/flosse/rust-web-framework-comparison
- Best Rust Web Frameworks (Encore) — https://encore.dev/articles/best-rust-web-frameworks
- The Current State of Rust Web Frameworks (LogRocket) — https://blog.logrocket.com/the-current-state-of-rust-web-frameworks/
- Axum — https://github.com/tokio-rs/axum
- SeaORM — https://www.sea-ql.org/SeaORM/
