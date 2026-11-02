# Temel Entity & Soft Delete

Çoğu tablo aynı birkaç column'ı paylaşır: bir primary key, oluşturma ve güncelleme zaman
damgaları ve — silinen row'ları etrafta tutuyorsan — bir `deleted_at` işareti. Rustali bunlara
iki trait aracılığıyla paylaşılan bir şekil verir, `BaseEntity` ve `SoftDelete`, ve `make:model`
iskeleti bunları senin için yazar; böylece her model aynı temelden başlar — diğer dillerde bir
temel sınıfa başvuracağın fikir, Rust'ın kalıtımı olmadığı için trait olarak ifade edilir.

## İskeletlenmiş model

`rustali make:model Post`, zaten ortak alanları taşıyan ve her iki trait'i de uygulayan bir model
üretir:

```rust
use rustali::{orm::chrono::{DateTime, Utc}, prelude::*};
use serde::Serialize;

#[derive(Entity, Debug, Clone, Serialize)]
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

Kendi column'larını anahtar ile zaman damgaları arasına eklersin. Zaman damgaları
`#[derive(Entity)]`'nin konvansiyonlarıyla yönetilir (`created_at` eklemede, `updated_at` her
kaydetmede — [Entity'ler → Zaman damgaları](entities#zaman-damgaları)'na bak); bu yüzden onları
asla elle ayarlamazsın.

## `BaseEntity`

`BaseEntity`, o alanlara sahip her model tarafından paylaşılan küçük bir erişimci trait'idir —
`id()`, `created_at()`, `updated_at()`. Rust'ın temel sınıfı yoktur; bu yüzden "bir temel model",
bir struct'ın genişlettiği bir ebeveyn değil, uyguladığı bir trait'tir; alanlar struct'ta kalır,
trait onlara paylaşılan, tip denetimli erişim verir.

Değeri **genel koddur**: bir fonksiyon, somut tipi adlandırmadan "herhangi bir temel modeli"
alabilir.

```rust
fn audit_line<T: BaseEntity<Id = i64>>(row: &T) -> String {
    format!("#{} created {}", row.id(), row.created_at())
}

audit_line(&post);     // BaseEntity uygulayan her model için çalışır
```

Onu elle uygula (yukarıdaki gibi) ya da iskeletin yazmasına izin ver. Ona yalnızca paylaşılan
işlemenin işe yaradığı modellerde ihtiyacın var — bir entity'nin çalışması için gerekli değildir.

## `SoftDelete`

`SoftDelete`, null olabilen bir `deleted_at` taşıyan modeller için `BaseEntity`'yi genişletir. Bir
erişimci ve bir kolaylık metodu ekler:

| Metot | Döndürdüğü |
|---|---|
| `deleted_at()` | `Option<DateTime<Utc>>` — row'un ne zaman çöpe atıldığı ya da canlıyken `None` |
| `is_trashed()` | `deleted_at` ayarlandığında `true` |

```rust
if post.is_trashed() {
    // satır soft-delete edilmiş
}
```

Trait yalnızca erişimci tarafıdır. **Davranış** — bir `delete`'i bir damgaya çevirmek ve çöpe
atılmış row'ları query'lerden gizlemek — yapılandırmayla yönlendirilir ve sırada anlatılır.

## Soft delete davranışı

Soft delete, varsayılan olarak kapalı, proje genelinde bir politikadır. Onu `.env`'de aç:

```bash
# .env
SOFT_DELETE=true
```

Açıkken, bir `deleted_at` column'ı olan herhangi bir model için:

- **`delete`, kaldırmak yerine damgalar.** `post.delete(&db)`, `deleted_at = now()` ayarlar ve bir
  `DELETE` göndermek yerine row'u tabloda bırakır.
- **Query'ler çöpe atılmış row'ları gizler.** `Post::query()` ve dolayısıyla `find`, `all` ve
  `count`, `deleted_at IS NULL`'a filtrelenmiş başlar; böylece çöpe atılmış row'lar sıradan
  okumalarda görünmez.

```rust
// soft_delete = true ile
let post = Post::find_or_404(&db, id).await?;
post.delete(&db).await?;                       // deleted_at damgalandı, satır tutuldu

assert!(Post::find(&db, id).await?.is_none());  // normal sorgulardan gizlendi
```

`soft_delete = false` ile (varsayılan), `delete` row'u kaldırır ve query'ler filtrelenmez —
`deleted_at` column'ı o zaman yalnızca kendin kullanabileceğin bir veridir.

Anahtar, yapılandırmadan başlangıçta bir kez okunur; bu yüzden tüm uygulama için çağrı başına bir
karar değil, tek, tutarlı bir politikadır. `deleted_at` column'ı olmayan bir model her iki durumda
da etkilenmez — `delete`'i her zaman gerçek bir silmedir.

## Çöpe atılmış row'ları görmek

Soft delete açıkken ve çöpe atılmış row'lara da ihtiyacın olduğunda — bir "yakında silindi"
view'ı, bir admin ekranı — o tek query için SeaORM'e in, çünkü entity'nin `query()`'si onları
filtreler:

```rust
use rustali::orm::sea_orm::EntityTrait;

// her satır, çöpe atılanlar dahil
let all = post_entity::Entity::find().all(&db).await?;
```

Bu bilinçli olarak açıktır: varsayılan yol güvenlidir (çöpe atılmış row'lar gizli kalır) ve
onları görmek bilinçli bir adımdır.

Sıradaki: [Migration'lar (Code-First)](migrations).
