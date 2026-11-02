# Entity'ler (Entity)

Bir entity, `#[derive(Entity)]` ile işaretlediğin **düz bir struct**'tan ibarettir. Etrafta
dolaştırdığın, serileştirdiğin ve render ettiğin şey bu struct'tır; derive ise [SeaORM] altyapısını
(`Model`, `ActiveModel`, column'lar, primary key) hiç dokunmadığın gizli bir modülde üretir —
geriye okunması kolay tek bir tip kalır, ortalıkta ORM standart kodu görünmez.

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

Modelin tamamı bu kadar. `#[derive(Entity)]` ayrıca modeli şema kayıt defterine bağlar; böylece
[`migrate:diff`](migrations) onu görebilir.

## Konvansiyonlar

Derive konvansiyonlara dayanır; böylece en yaygın durumda hiçbir attribute yazmana gerek kalmaz:

- **`id`** adlı bir alan primary key olur. Tamsayı türünde bir anahtar **otomatik artar**.
- **`Option<T>`** alanları null olabilir; geri kalan her şey `NOT NULL` olur.
- `DateTime` tipindeki **`created_at`** / **`updated_at`** alanları otomatik ayarlanır —
  `created_at` eklemede, `updated_at` ise her kaydetmede.
- Tablo adı varsayılan olarak struct adının snake_case hâli + `s`'tir; `#[table]` ile değiştirebilirsin.

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
| `#[key]` | Primary key'i işaretler (alan adı `id` değilken). |
| `#[key(auto = false)]` | Otomatik artmayan anahtar. |
| `#[column(name = "col")]` | Farklı adlı bir column'a eşler. |
| `#[column(unique)]` | `UNIQUE` kısıtlaması ekler. |
| `#[column(max_len = N)]` | Metin alanı için `VARCHAR(N)`. |
| `#[column(index)]` | Column'ın indeksleneceğini kaydeder (şema araçları için). |
| `#[skip]` | Saklanmaz; `Default` uygulaması gerektirir (yüklemede doldurulur). |

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

`#[derive(Entity)]`, struct'a bir CRUD arayüzü ve tipli column sabitleri kazandırır. Hepsi bir
bağlantı alır — [`Db`](queries#db-extractor-ı) extractor'ı ya da `&*ctx.db()?`.

| Metot | Yaptığı |
|---|---|
| `Todo::query()` | Bir [`Query`](queries) builder'ı başlatır. |
| `Todo::find(&db, id)` | Primary key'e göre `Option<Todo>`. |
| `Todo::find_or_404(&db, id)` | `Todo` ya da `Err(Error::NotFound)`. |
| `Todo::all(&db)` | Primary key'e göre sıralı tüm row'lar. |
| `Todo::count(&db)` | Row sayısı. |
| `todo.insert(&db)` | Ekler; saklanan row'u döndürür (üretilen id ve zaman damgalarıyla). |
| `todo.update(&db)` | Tüm column'ları günceller; saklanan row'u döndürür. |
| `todo.delete(&db)` | Bu row'u siler. |
| `Todo::ID`, `Todo::TITLE`, ... | Query'ler için tipli column handle'ları. |

Bunların her biri, kullanımları ve dönüş tipleriyle birlikte [Model Metotları](model-methods)
sayfasında tek tek ele alınır.

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
Örneklerde, varsayılanları ayarlayan ve `id` ile zaman damgalarını `insert`'e bırakan küçük bir
`new` tanımlanır.)

## Bir handler'da bir entity kullanmak

```rust
#[get("/{id}")]
async fn show(db: Db, Path(id): Path<i64>) -> Result<Json<Todo>> {
    Ok(Json(Todo::find_or_404(&db, id).await?))
}
```

Struct `Serialize`'ı türettiğinden `Json<Todo>` döndürmek sorunsuz çalışır.

## Relation'lar

Diğer tablolara bağlantılar — bir gönderinin yorumları, bir yorumun gönderisi, bir gönderinin
etiketleri — konteyner attribute'larıyla (`#[has_many]`, `#[belongs_to]`, `#[has_one]`,
`#[many_to_many]`) bildirilir ve bunlar için üretilen açık metotlarla yüklenir. Tam rehber için
[Relation'lar](relations/)'a bak.

## Derive'in ürettiği şey

Buradaki hiçbir şey reflection ya da çalışma zamanı sihri değildir — `#[derive(Entity)]`, `cargo
expand` ile okuyabileceğin sıradan koda açılır. Bir `Todo` struct'ı için `todo_entity` adlı gizli
bir modülde şunları üretir:

- **SeaORM entity'si**: `Model`, `ActiveModel`, `Column`, `PrimaryKey`, `Relation` — SeaORM'ün
  ihtiyaç duyduğu altyapı, gözünün önünden uzak tutulur.
- Her iki yönde de **`From` dönüşümleri**: `From<todo_entity::Model> for Todo` ve
  `From<Todo> for todo_entity::Model`. Query sonuçları senin struct'ın olarak geri gelir, çünkü
  terminal metotlar senin yerine `.into()` çağırır.
- Tablo adını, column üst verisini (`COLUMNS`), anahtar erişimcisini ve active-model builder'larını
  taşıyan bir **`impl Entity for Todo`** — şema kayıt defterinin ve [migration'ların](migrations)
  okuduğu şey budur.
- Yukarıdaki tablodaki **inherent metotlar**, tipli column sabitleri ve varsa
  [relation](relations/) erişimcileri.

Eşleme düpedüz `From`'dan ibaret olduğu için, bir kat aşağı inmen gerektiğinde struct'ınla ham
SeaORM modeli arasında her zaman dönüşüm yapabilirsin.

## Zaman damgaları

Bir struct'ta `DateTime` tipinde `created_at` ve/veya `updated_at` alanı varsa, derive bir kaydetme
kancası (SeaORM'ün `ActiveModelBehavior::before_save`) üretir:

- `created_at`, henüz ayarlanmamışsa yalnızca bir kez, **eklemede** ayarlanır.
- `updated_at`, **her** kaydetmede ayarlanır (hem eklemede hem güncellemede).

Yani bir row oluştururken ikisini de istediğin yer tutucu değerde bırakabilirsin — `insert`, gerçek
zaman damgaları doldurulmuş saklanan row'u geri verir. Eklemede her iki alan da struct'tan
gönderilmek yerine kancaya bırakılır; güncellemede `updated_at` her zaman tazelenir. Alanları
tamamen çıkarırsan hiç zaman damgan olmaz; kanca yalnızca bu alanlar varsa üretilir.

## Paylaşılan temel alanlar & soft delete

`rustali make:model`, her yeni modele konvansiyonel `id` / `created_at` / `updated_at` /
`deleted_at` alanlarıyla birlikte `BaseEntity` ve `SoftDelete` trait uygulamalarını verir — yani
paylaşılan bir temel model. Yapılandırmada soft delete açıldığında `delete`, row'u silmek yerine
`deleted_at`'i damgalar ve query'ler çöpe atılmış row'ları gizler. [Temel Entity & Soft
Delete](base-entity)'e bak.

## SeaORM'e inmek

Üretilen column sabitleri SeaORM'ün trait'lerini uygular ve
[`Query::into_select()`](queries) sana ham `Select`'i verir — yani bu kolaylıkların kapsamadığı her
şeyi `rustali::orm::sea_orm` üzerinden doğrudan SeaORM ile yine de yapabilirsin. Öğrenmen gereken
ikinci bir query dili yoktur.

Sıradaki: [Query'ler & Pagination](queries).
