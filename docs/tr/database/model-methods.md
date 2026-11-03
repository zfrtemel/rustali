# Model Metotları

`#[entity]`, her modele yaygın veritabanı işlemleri için bir dizi metot kazandırır —
row'ları okumak, oluşturmak, güncellemek ve silmek. Bu sayfa her birini tek tek ele alır: ne aldığı,
ne döndürdüğü ve ne zaman kullanacağın.

Her metot ilk argümanı olarak bir veritabanı bağlantısı alır (örnek metotlarda ise örtük `self`'in
ardından gelen ilk argüman olarak). Bir handler'da bu bağlantı [`Db`](queries#db-extractor-ı)
extractor'ıdır; başka yerlerde `&*ctx.db()?`'dir. Hepsi `async`'tir ve
[`Result`](../basics/responses#hatalar-degerdir) döndürür; bu yüzden onları `.await?` ile
çağırırsın.

Aşağıdaki örnekler şu modeli kullanır:

```rust
use rustali::{orm::chrono::{DateTime, Utc}, prelude::*};

#[entity]
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

Primary key'e göre bir row getirir. O anahtara sahip bir row yoksa hata değil, `None` döndürür;
yani kaydın bulunmamasının ne anlama geldiğine sen karar verirsin:

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

`find` gibidir, ama kayıt bulunamadığında `None` yerine `Err(Error::NotFound)` döner. Bir handler'da
`?`, bunu doğrudan bir 404 response'una çevirir — "bu kaydı göster" türünde bir route için tam da
istediğin davranış:

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

Primary key'e göre sıralı tüm row'lar. Küçük tablolar için uygundur; büyüme ihtimali olan her şeyde
ise tüm tabloyu belleğe yüklemek yerine [`query`](#query) ile filtrele ya da sayfalara böl.

```rust
let todos = Todo::all(&db).await?;
println!("{} todos", todos.len());
```

### `count`

```rust
Todo::count(&db) -> Result<u64>
```

Tablodaki row sayısı; sayımı veritabanı yapar, hiçbir row belleğe yüklenmez.

```rust
if Todo::count(&db).await? == 0 {
    println!("nothing to do");
}
```

### `query`

```rust
Todo::query() -> Query<Todo>
```

Filtreleme, sıralama, limit ya da pagination gerektiğinde — kısacası "id'ye göre" ya da "tümü"nün
ötesindeki her durumda — tipli bir [query builder'ı](queries) başlatır. Bu metot senkrondur
(yalnızca query'yi kurar); query'yi asıl çalıştıran, `async` olan terminal metottur:

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

Değeri yeni bir row olarak ekler ve **saklanan** row'u döndürür. Dönen değerde gerçek üretilen
anahtar ve zaman damgaları doldurulmuş olur; bu yüzden gönderdiğin değeri değil, `insert`'in geri
verdiği değeri kullan:

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

Otomatik artan anahtar ile `created_at` / `updated_at` alanları senin yerine yönetilir ([Entity'ler →
Zaman damgaları](entities#zaman-damgaları)'na bak); yukarıdaki yer tutucu değerlerin önemsiz
olmasının nedeni de budur.

## Modelleri güncellemek

### `update`

```rust
todo.update(&db) -> Result<Todo>
```

Değerin tüm column'larını ilgili row'a geri yazar (eşleştirme primary key'e göre yapılır) ve saklanan
row'u döndürür. `updated_at` otomatik tazelenir. Olağan akış şudur: yükle, alanları değiştir, kaydet:

```rust
let mut todo = Todo::find_or_404(&db, id).await?;
todo.done = true;
let todo = todo.update(&db).await?;     // updated_at artırıldı
```

`update`, yalnızca değiştirdiğin alanları değil **tüm** column'ları yazar — yani bu, satırın tamamını
güncelleyen bir işlemdir. Bu yüzden kaydettiğin değer eksiksiz ve güncel bir row olmalıdır (önce
yüklediysen zaten öyledir).

## Modelleri silmek

### `delete`

```rust
todo.delete(&db) -> Result<()>
```

Row'u siler (eşleştirme primary key'e göre yapılır). Silinen row artık bir şeye karşılık gelmediği
için metot değeri tüketir:

```rust
let todo = Todo::find_or_404(&db, id).await?;
todo.delete(&db).await?;
```

Row'u önce yüklemeden silmek istiyorsan, silmeyi [query builder'ı](queries) üzerinden çalıştır ya da
SeaORM'e in — buradaki `delete`, "row zaten elimde" durumunun yoludur.

## Relation'lar

İlişkili row'lar, kendileri için üretilen metotlarla yüklenir — `todo.comments(&db)` ve istekli
(eager) biçimi `Todo::with_comments(todos, &db)` — bildirdiğin her relation için birer tane. Bunlar
[Relation'lar](relations/) bölümünde ayrıntılı olarak ele alınır.

## Tipli column handle'ları

Metotların yanı sıra derive, query'lerde kullanman için her column'a bir sabit üretir — `Todo::ID`,
`Todo::TITLE`, `Todo::DONE`, … (`Todo::DONE.eq(false)` gibi). Bir query'de bir column'a metin yerine
tip denetiminden geçmiş bir adla başvurmanın yolu bunlardır.
[Query'ler](queries#bir-query-olusturmak)'e bak.

## SeaORM'e inmek

Bu metotlar gündelik durumları karşılar. Karşılamadıkları her şey için — toplu güncellemeler,
upsert'ler, özel join'ler — [`Query::into_select()`](queries#seaorm-e-kacmak) sana ham SeaORM
`Select`'ini verir; ayrıca `rustali::orm::sea_orm`, SeaORM API'sinin tamamını yeniden dışa aktarır.
Öğrenmen gereken ikinci bir query dili yoktur.
