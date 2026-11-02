# Model Metotları

`#[derive(Entity)]`, her modele yaygın veritabanı işlemleri için bir dizi metot verir — row'ları
okumak, oluşturmak, güncellemek ve silmek. Bu sayfa her birini tek tek gezer: ne aldığı, ne
döndürdüğü ve ne zaman ona başvuracağın.

Her metot, ilk argümanı olarak (ya da örnek metotlar için örtük `self` artı bir) bir veritabanı
bağlantısı alır. Bir handler'da o bağlantı [`Db`](queries#db-extractor-ı) extractor'ıdır; başka
yerlerde `&*ctx.db()?`'dir. Hepsi `async`'tir ve
[`Result`](../basics/responses#hatalar-degerdir) döndürür; bu yüzden onları `.await?`'larsın.

Aşağıdaki örnekler bu modeli kullanır:

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

## Modelleri getirmek

### `find`

```rust
Todo::find(&db, id) -> Result<Option<Todo>>
```

Primary key'e göre bir row getir. O anahtara sahip hiçbir row olmadığında `None` döndürür
— bir hata değil; bu yüzden bir eksikliğin ne anlama geldiğine sen karar verirsin:

```rust
match Todo::find(&db, 1).await? {
    Some(todo) => println!("got {}", todo.title),
    None => println!("no such todo"),
}
```

### `find_or_404`

```rust
Todo::find_or_404(&db, id) -> Result<Todo>
```

`find` gibi, ama bir eksiklik `None` yerine `Err(Error::NotFound)`'dur. Bir handler'da `?` bunu
doğrudan bir 404 response'una çevirir — ki bu, bir "bu kaydı göster" route'u için tam istediğin şey:

```rust
#[get("/{id}")]
async fn show(db: Db, Path(id): Path<i64>) -> Result<Json<Todo>> {
    Ok(Json(Todo::find_or_404(&db, id).await?))   // eksikse 404
}
```

### `all`

```rust
Todo::all(&db) -> Result<Vec<Todo>>
```

Primary key'e göre sıralı her row. Küçük tablolar için iyi; büyüyebilen herhangi bir şey
için, tüm tabloyu belleğe yüklemek yerine [`query`](#query) ile filtrele ya da sayfala.

```rust
let todos = Todo::all(&db).await?;
println!("{} todos", todos.len());
```

### `count`

```rust
Todo::count(&db) -> Result<u64>
```

Tablodaki row sayısı, veritabanı tarafından sayılır — hiçbir row yüklemez.

```rust
if Todo::count(&db).await? == 0 {
    println!("nothing to do");
}
```

### `query`

```rust
Todo::query() -> Query<Todo>
```

Filtreleme, sıralama, limit ya da pagination'a ihtiyacın olduğunda — "id'ye göre" ya da "tümü"nün
ötesindeki her şey — tipli bir [query builder'ı](queries) başlat. Senkrondur (query'yi kurar);
onu çalıştıran terminal metot `async` olandır:

```rust
let open = Todo::query()
    .filter(Todo::DONE.eq(false))
    .order_by_desc(Todo::CREATED_AT)
    .limit(20)
    .fetch_all(&db)
    .await?;
```

Tam builder için [Query'ler & Pagination](queries)'a bak.

## Modelleri oluşturmak

### `insert`

```rust
todo.insert(&db) -> Result<Todo>
```

Değeri yeni bir row olarak ekle ve **saklanan** row'u döndür. Döndürülen değerin gerçek
üretilen anahtarı ve zaman damgaları doldurulmuştur; bu yüzden geçirdiğin değeri değil, `insert`'in
geri verdiğini kullan:

```rust
let todo = Todo {
    id: 0,                              // yok sayılır — DB atar
    title: "write docs".into(),
    done: false,
    created_at: DateTime::<Utc>::MIN_UTC,  // yok sayılır — eklemede ayarlanır
    updated_at: DateTime::<Utc>::MIN_UTC,
};
let todo = todo.insert(&db).await?;     // todo.id ve zaman damgaları artık gerçek
```

Otomatik artan bir anahtar ve `created_at` / `updated_at` alanları senin için yönetilir
([Entity'ler → Zaman damgaları](entities#zaman-damgaları)'na bak); yukarıdaki yer tutucu
değerlerinin önemsiz olmasının nedeni budur.

## Modelleri güncellemek

### `update`

```rust
todo.update(&db) -> Result<Todo>
```

Değerin her column'ını row'una geri yaz (primary key'e göre eşleştirilir) ve saklanan row'u
döndür. `updated_at` otomatik olarak yenilenir. Olağan şekil yükle, alanları değiştir, kaydet'tir:

```rust
let mut todo = Todo::find_or_404(&db, id).await?;
todo.done = true;
let todo = todo.update(&db).await?;     // updated_at artırıldı
```

`update`, yalnızca değiştirdiklerini değil, **tüm** column'ları yazar — bu bir tam row
güncellemesidir; bu yüzden kaydettiğin değer tam, güncel bir row olmalıdır (önce yüklediysen,
öyledir).

## Modelleri silmek

### `delete`

```rust
todo.delete(&db) -> Result<()>
```

Row'u sil (primary key'e göre eşleştirilir). Artık hiçbir şeye atıfta bulunmadığından değeri
tüketir:

```rust
let todo = Todo::find_or_404(&db, id).await?;
todo.delete(&db).await?;
```

Önce yüklemeden silmek için, bir silmeyi [query builder'ı](queries) aracılığıyla çalıştır ya da
SeaORM'e in — buradaki `delete`, "row zaten bende" yoludur.

## Relation'lar

İlişkili row'lar, kendi üretilen metotlarıyla yüklenir — `todo.comments(&db)` ve istekli (eager)
`Todo::with_comments(todos, &db)` — bildirdiğin her relation için bir tane. Bunlar
[Relation'lar](relations/) altında tam olarak ele alınmıştır.

## Tipli column handle'ları

Metotların yanı sıra, derive column başına bir sabit üretir — `Todo::ID`, `Todo::TITLE`,
`Todo::DONE`, … — query'lerde kullanmak için (`Todo::DONE.eq(false)`). Bir query'nin bir column'a bir
dize yerine denetlenmiş bir adla atıfta bulunma yolu bunlardır.
[Query'ler](queries#bir-query-olusturmak)'e bak.

## SeaORM'e inmek

Bu metotlar gündelik durumları kapsar. Kapsamadıkları her şey için — toplu güncellemeler,
upsert'ler, özel join'ler — [`Query::into_select()`](queries#seaorm-e-kacmak) sana ham SeaORM
`Select`'ini verir ve `rustali::orm::sea_orm` tüm SeaORM API'sini yeniden dışa aktarır.
Öğrenilecek ikinci bir query dili yoktur.
