# Temel Entity & Soft Delete

Çoğu tablo aynı birkaç column'ı paylaşır: bir primary key, oluşturma ve güncelleme zaman damgaları
ve — silinen row'ları saklıyorsan — bir `deleted_at` işareti. Rustali bu column'lara `BaseEntity` ve
`SoftDelete` adlı iki trait ile ortak bir biçim kazandırır; `make:model` iskeleti de bunları senin
yerine yazar. Böylece her model aynı temelden başlar — başka dillerde bir temel sınıftan
türeteceğin bu yapı, Rust'ta kalıtım olmadığı için trait olarak ifade edilir.

## İskeletlenmiş model

`rustali make:model Post`, ortak alanları hazır taşıyan ve her iki trait'i de uygulayan bir model
üretir:

```rust
use rustali::{orm::chrono::{DateTime, Utc}, prelude::*};

#[entity]
#[table("posts")]
pub struct Post {
    pub id: i64,
    // Kendi alanlarını buraya ekle.
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl BaseEntity for Post {
    type Id = i64;
    fn id(&self) -> Self::Id { self.id }
    fn created_at(&self) -> DateTime<Utc> { self.created_at }
    fn updated_at(&self) -> DateTime<Utc> { self.updated_at }
}

impl SoftDelete for Post {
    fn deleted_at(&self) -> Option<DateTime<Utc>> { self.deleted_at }
}
```

Kendi column'larını anahtarla zaman damgalarının arasına eklersin. Zaman damgaları
`#[entity]`'nin konvansiyonlarıyla yönetilir (`created_at` eklemede, `updated_at` her
kaydetmede — [Entity'ler → Zaman damgaları](entities#zaman-damgaları)'na bak); bu yüzden onları hiç
elle ayarlamazsın.

## `BaseEntity`

`BaseEntity`, bu alanlara sahip tüm modellerin paylaştığı küçük bir erişimci trait'idir —
`id()`, `created_at()`, `updated_at()`. Rust'ta temel sınıf kavramı yoktur; bu yüzden burada "temel
model", bir struct'ın türediği bir ebeveyn değil, uyguladığı bir trait'tir. Alanlar struct'ta kalır,
trait ise onlara paylaşılan ve tip denetimli bir erişim sağlar.

Asıl değeri **jenerik kodda** ortaya çıkar: bir fonksiyon, somut tipi adıyla belirtmeden "herhangi
bir temel model" alabilir.

```rust
fn audit_line<T: BaseEntity<Id = i64>>(row: &T) -> String {
    format!("#{} created {}", row.id(), row.created_at())
}

audit_line(&post);     // BaseEntity uygulayan tüm modeller için çalışır
```

Trait'i (yukarıdaki gibi) elle uygulayabilir ya da iskeletin senin yerine yazmasına bırakabilirsin.
Ona yalnızca paylaşılan işlemelerden fayda sağlayan modellerde ihtiyaç duyarsın — bir entity'nin
çalışması için şart değildir.

## `SoftDelete`

`SoftDelete`, null olabilen bir `deleted_at` alanı taşıyan modeller için `BaseEntity`'yi genişletir.
Bir erişimci ile bir kolaylık metodu ekler:

| Metot | Döndürdüğü |
|---|---|
| `deleted_at()` | `Option<DateTime<Utc>>` — row'un ne zaman çöpe atıldığı, canlıysa `None` |
| `is_trashed()` | `deleted_at` ayarlıysa `true` |

```rust
if post.is_trashed() {
    // satır soft-delete edilmiş
}
```

Trait yalnızca erişim tarafını sağlar. **Asıl davranış** — bir `delete`'i damgalamaya çevirmek ve
çöpe atılmış row'ları query'lerden gizlemek — yapılandırmayla belirlenir; onu da hemen aşağıda
anlatıyoruz.

## Soft delete davranışı

Soft delete, varsayılan olarak kapalı olan, proje genelinde bir politikadır. Onu `.env`'de açarsın:

```bash
# .env
SOFT_DELETE=true
```

Açıkken, `deleted_at` column'ı olan her model için şunlar geçerli olur:

- **`delete`, silmek yerine damgalar.** `post.delete(&db)`, `deleted_at = now()` yapar ve bir
  `DELETE` göndermek yerine row'u tabloda bırakır.
- **Query'ler çöpe atılmış row'ları gizler.** `Post::query()` — ve dolayısıyla `find`, `all` ve
  `count` — `deleted_at IS NULL` filtresiyle başlar; böylece çöpe atılmış row'lar sıradan okumalarda
  görünmez.

```rust
// soft_delete = true ile
let post = Post::find_or_404(&db, id).await?;
post.delete(&db).await?;                       // deleted_at damgalandı, satır tutuldu

assert!(Post::find(&db, id).await?.is_none());  // normal sorgulardan gizlendi
```

`soft_delete = false` iken (varsayılan), `delete` row'u gerçekten siler ve query'lere filtre
uygulanmaz — bu durumda `deleted_at` column'ı yalnızca senin dilediğin gibi kullanabileceğin sıradan
bir veridir.

Bu ayar başlangıçta yapılandırmadan bir kez okunur; yani çağrı başına verilen bir karar değil, tüm
uygulama için tek ve tutarlı bir politikadır. `deleted_at` column'ı olmayan bir model iki durumdan da
etkilenmez — onun `delete`'i her zaman gerçek bir silmedir.

## Çöpe atılmış row'ları görmek

Soft delete açıkken çöpe atılmış row'lara da ihtiyacın olduğunda — bir "yakında silinenler" view'ı,
bir admin ekranı gibi — o tek query için SeaORM'e inmen gerekir, çünkü entity'nin `query()`'si onları
zaten filtreler:

```rust
use rustali::orm::sea_orm::EntityTrait;

// her satır, çöpe atılanlar dahil
let all = post_entity::Entity::find().all(&db).await?;
```

Bu, bilerek böyle tasarlanmıştır: varsayılan yol güvenlidir (çöpe atılmış row'lar gizli kalır) ve
onları görmek ayrı, bilinçli bir adım gerektirir.

Sıradaki: [Migration'lar (Code-First)](migrations).
