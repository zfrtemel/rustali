# Entity'ler (Entity)

Bir entity, `#[derive(Entity)]` ile işaretlediğin bir **düz struct**'tır. Etrafta dolaştırdığın,
serileştirdiğin ve render ettiğin şey struct'tır; derive, [SeaORM] altyapısını (`Model`,
`ActiveModel`, column'lar, primary key) asla dokunmadığın gizli bir modüle üretir — okunabilir
tek bir tip, görünür ORM standart kodu yok.

[SeaORM]: https://www.sea-ql.org/SeaORM/

## Bir tane tanımlamak

```rust
use rustali::{orm::chrono::{DateTime, Utc}, prelude::*};
use serde::Serialize;

#[derive(Entity, Debug, Clone, Serialize)]
#[table("todos")]
pub struct Todo {
    pub id: i64,
    #[column(max_len = 200)]
    pub title: String,
    pub done: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Tüm model bu kadar. `#[derive(Entity)]` ayrıca onu şema kayıt defterine bağlar; böylece
[`migrate:diff`](migrations) onu görebilir.

## Konvansiyonlar

Derive, konvansiyonlara dayanır; böylece yaygın durum hiçbir attribute gerektirmez:

- **`id`** adlı bir alan primary key'dir. Bir tamsayı anahtarı **otomatik artar**.
- **`Option<T>`** alanları null olabilir; geri kalan her şey `NOT NULL`'dur.
- Bir `DateTime` tipindeki **`created_at`** / **`updated_at`** adlı alanlar otomatik ayarlanır —
  `created_at` eklemede, `updated_at` her kaydetmede.
- Tablo adı varsayılan olarak snake_case struct adı + `s`'dir; `#[table]` ile geçersiz kıl.

## Attribute'lar

### Konteyner

| Attribute | Etkisi |
|---|---|
| `#[table("name")]` | Tablo adı. |
| `#[entity(crate = "rustali_orm")]` | Üretilen kod için ORM crate'inin yolu (nadiren gerekir). |
| `#[has_many(...)]` | Bire-çok relation. [Relation'lar](relations/)'a bak. |
| `#[belongs_to(...)]` | Bire-çok relation'ın çocuk tarafı. |
| `#[has_one(...)]` | Bire-bir relation. |
| `#[many_to_many(...)]` | Bir bağlantı tablosu üzerinden çoka-çok relation. |

### Alan

| Attribute | Etkisi |
|---|---|
| `#[key]` | Primary key'i işaretle (`id` adlı olmadığında). |
| `#[key(auto = false)]` | Otomatik artmayan anahtar. |
| `#[column(name = "col")]` | Farklı adlı bir column'a eşle. |
| `#[column(unique)]` | Bir `UNIQUE` kısıtlaması ekle. |
| `#[column(max_len = N)]` | Bir dize için `VARCHAR(N)`. |
| `#[column(index)]` | Column'ın indekslenmesi gerektiğini kaydet (şema araçları için). |
| `#[skip]` | Saklanmaz; `Default`'u uygulamalı (yüklemede doldurulur). |

```rust
#[derive(Entity, Clone)]
#[table("users")]
pub struct User {
    pub id: i64,
    #[column(unique)]
    pub email: String,
    pub password_hash: String,
    #[skip]
    pub transient_token: Option<String>,
}
```

## Üretilen metotlar

`#[derive(Entity)]`, struct'a bir CRUD yüzeyi ve tipli column sabitleri verir. Hepsi bir bağlantı
alır — [`Db`](queries#db-extractor-ı) extractor'ı ya da `&*ctx.db()?`.

| Metot | Yaptığı |
|---|---|
| `Todo::query()` | Bir [`Query`](queries) builder'ı başlat. |
| `Todo::find(&db, id)` | Primary key'e göre `Option<Todo>`. |
| `Todo::find_or_404(&db, id)` | `Todo` ya da `Err(Error::NotFound)`. |
| `Todo::all(&db)` | Primary key'e göre her row. |
| `Todo::count(&db)` | Row sayısı. |
| `todo.insert(&db)` | Ekle; saklanan row'u döndürür (üretilen id + zaman damgalarıyla). |
| `todo.update(&db)` | Her column'ı güncelle; saklanan row'u döndürür. |
| `todo.delete(&db)` | Bu row'u sil. |
| `Todo::ID`, `Todo::TITLE`, ... | Query'ler için tipli column handle'ları. |

Bunların her biri, kullanım ve dönüş tipleriyle, [Model Metotları](model-methods)'nda metot metot
ele alınmıştır.

```rust
// oluştur
let todo = Todo::new("write docs").insert(&db).await?;

// oku
let one = Todo::find_or_404(&db, todo.id).await?;

// güncelle
let mut t = one;
t.done = true;
let t = t.update(&db).await?;

// sil
t.delete(&db).await?;
```

(Yukarıdaki `Todo::new(...)`, senin yazdığın sıradan bir kurucudur — derive onu üretmez.
Örnekler, varsayılanları ayarlayan ve `id`/zaman damgalarını `insert`'e bırakan küçük bir `new`
tanımlar.)

## Bir handler'da bir entity kullanmak

```rust
#[get("/{id}")]
async fn show(db: Db, Path(id): Path<i64>) -> Result<Json<Todo>> {
    Ok(Json(Todo::find_or_404(&db, id).await?))
}
```

Struct `Serialize`'ı türettiği için, `Json<Todo>` döndürmek sorunsuz çalışır.

## Relation'lar

Diğer tablolara bağlantılar — bir gönderinin yorumları, bir yorumun gönderisi, bir gönderinin
etiketleri — konteyner attribute'larıyla (`#[has_many]`, `#[belongs_to]`, `#[has_one]`,
`#[many_to_many]`) bildirilir ve üretilen, açık metotlarla yüklenir. Tam rehber için
[Relation'lar](relations/)'a bak.

## Derive'in ürettiği şey

Buradaki hiçbir şey yansıma (reflection) ya da çalışma zamanı sihri değil — `#[derive(Entity)]`,
`cargo expand` ile okuyabileceğin sıradan koda genişler. Bir `Todo` struct'ı için, `todo_entity`
adlı gizli bir modüle şunları yayar:

- **SeaORM entity'si**: `Model`, `ActiveModel`, `Column`, `PrimaryKey`, `Relation` — SeaORM'ün
  ihtiyaç duyduğu altyapı, gözden uzak tutulur.
- Her iki yönde **`From` dönüşümleri**: `From<todo_entity::Model> for Todo` ve
  `From<Todo> for todo_entity::Model`. Query sonuçları struct'ın olarak geri gelir çünkü terminal
  metotlar senin için `.into()` çağırır.
- Tablo adını, column üst verisini (`COLUMNS`), anahtar erişimcisini ve active-model builder'larını
  taşıyan bir **`impl Entity for Todo`** — şema kayıt defterinin ve [migration'ların](migrations)
  okuduğu şey budur.
- Yukarıdaki tablodaki **inherent metotlar**, tipli column sabitleri ve varsa herhangi bir
  [relation](relations/) erişimcisi.

Eşleme düz `From` olduğu için, bir seviye aşağı inmen gerektiğinde struct'ın ile ham SeaORM modeli
arasında her zaman dönüştürme yapabilirsin.

## Zaman damgaları

Bir struct'ın bir `DateTime` tipindeki `created_at` ve/veya `updated_at` alanı olduğunda, derive
bir kaydetme kancası üretir (SeaORM'ün `ActiveModelBehavior::before_save`):

- `created_at`, ayarlanmamışsa, bir kez, **eklemede** ayarlanır.
- `updated_at`, **her** kaydetmede ayarlanır (ekleme ve güncelleme).

Yani bir row kurarken ikisini de herhangi bir yer tutucu değerde bırakabilirsin — `insert`,
gerçek zaman damgaları doldurulmuş saklanan row'u döndürür. Eklemede, her iki alan da struct'tan
gönderilmek yerine kancaya bırakılır; güncellemede, `updated_at` her zaman yenilenir. Alanları
tamamen atla, basitçe hiç zaman damgan olmaz; kanca yalnızca onlar var olduğunda üretilir.

## Paylaşılan temel alanlar & soft delete

`rustali make:model`, her yeni modele konvansiyonel `id` / `created_at` / `updated_at` /
`deleted_at` alanlarını artı `BaseEntity` ve `SoftDelete` trait uygulamalarını verir — paylaşılan
bir temel model. Yapılandırmada soft delete etkinleştirildiğinde, `delete` ardından row'u
kaldırmak yerine `deleted_at` damgalar ve query'ler çöpe atılmış row'ları gizler.
[Temel Entity & Soft Delete](base-entity)'e bak.

## SeaORM'e inmek

Üretilen column sabitleri SeaORM'ün trait'lerini uygular ve
[`Query::into_select()`](queries), sana ham `Select`'i verir — yani şekerin kapsamadığı her şeyi
`rustali::orm::sea_orm` aracılığıyla doğrudan SeaORM ile yine de yapabilirsin. Öğrenilecek ikinci
bir query dili yoktur.

Sıradaki: [Query'ler & Pagination](queries).
