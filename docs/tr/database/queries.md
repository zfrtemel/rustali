# Query'ler

`Todo::query()`, tipli bir `Query<Todo>` builder'ı döndürür. SeaORM'ün tanıdık filtre, sıralama ve
seçim metotlarını iletir, ama terminal metotları bir SeaORM modeli değil, **struct'ını** döndürür
— böylece düz-Rust-değeri diyarında kalırsın.

## `Db` extractor'ı

Handler'lar bir bağlantıyı `Db` isteyerek alır. SeaORM'ün `ConnectionTrait`'ini uygular; böylece
`&db` doğrudan herhangi bir query metoduna girer:

```rust
#[get("/")]
async fn index(db: Db) -> Result<Json<Vec<Todo>>> {
    Ok(Json(Todo::all(&db).await?))
}
```

Bir handler'ın dışında (bir işte, bir komutta), paylaşılan `Database`'i döndüren `ctx.db()?`'yi
kullan; `&*ctx.db()?` geçir.

## Bir query oluşturmak

Derive'in ürettiği tipli column sabitleriyle filtrele ve sırala:

```rust
let done = Todo::query()
    .filter(Todo::DONE.eq(true))
    .order_by_desc(Todo::CREATED_AT)
    .limit(10)
    .fetch_all(&db)
    .await?;
```

`filter`, `order_by_asc` / `order_by_desc`, `limit`, `offset` ve diğer builder metotları SeaORM'den
gelir (`ColumnTrait`, `QueryOrder`, `QuerySelect`); column sabitleri (`Todo::DONE`,
`Todo::CREATED_AT`) her alan için üretilir.

## Derinlemesine filtreleme

Her column sabiti bir SeaORM `ColumnTrait`'tir; bu yüzden karşılaştırma yardımcılarının tam kümesi
onda kullanılabilir:

```rust
Todo::TITLE.eq("write docs")          // =
Todo::TITLE.ne("skip")                // <>
Todo::TITLE.like("%docs%")            // LIKE
Todo::TITLE.contains("doc")           // LIKE '%doc%'
Todo::TITLE.starts_with("wr")         // LIKE 'wr%'
Todo::PRIORITY.gt(3)                  // >
Todo::PRIORITY.gte(3)                 // >=
Todo::PRIORITY.lt(10)                 // <
Todo::PRIORITY.is_in([1, 2, 3])       // IN (...)
Todo::NOTE.is_null()                  // IS NULL
Todo::NOTE.is_not_null()              // IS NOT NULL
Todo::DUE.between(start, end)         // BETWEEN
```

### Koşulları birleştirmek

`.filter(...)`'ı birden fazla kez çağır, koşullar **AND**'lenir:

```rust
let urgent = Todo::query()
    .filter(Todo::DONE.eq(false))
    .filter(Todo::PRIORITY.gte(8))   // done = false AND priority >= 8
    .fetch_all(&db)
    .await?;
```

**OR** (ve iç içe gruplar) için, bir SeaORM `Condition` kur ve onu tek bir `.filter(...)`'a geçir:

```rust
use rustali::orm::sea_orm::Condition;

let matches = Todo::query()
    .filter(
        Condition::any()                     // OR
            .add(Todo::TITLE.contains("doc"))
            .add(Todo::TITLE.contains("test")),
    )
    .filter(Todo::DONE.eq(false))            // ... AND done = false
    .fetch_all(&db)
    .await?;
```

`Condition::all()`, AND karşılığıdır; herhangi bir boolean ağacı kurmak için onları serbestçe iç
içe geçir.

## Relation'ları yüklemek

Relation'lar bir query'yle yüklenmez — entity'deki üretilen erişimci metotlarıyla (row başına tembel)
ya da `with_*` toplu yükleyicileriyle (istekli) yüklenir. Olağan desen, bir query çalıştırmak,
sonra N+1'i önlemek için istekli yüklemektir:

```rust
let posts = Post::query()
    .filter(Post::PUBLISHED.eq(true))
    .order_by_desc(Post::CREATED_AT)
    .fetch_all(&db)
    .await?;                                  // 1 sorgu

let with_comments = Post::with_comments(posts, &db).await?; // 1 sorgu daha
```

Tam rehber için [Relation'lar](relations/)'a bak.

## Getirme

| Terminal metot | Döndürdüğü |
|---|---|
| `fetch_all(&db)` | `Vec<Todo>` |
| `fetch_one(&db)` | `Option<Todo>` (ilk eşleşme) |
| `fetch_one_or_404(&db)` | `Todo` ya da `Err(Error::NotFound)` |
| `count(&db)` | `u64` |
| `exists(&db)` | `bool` |
| `paginate(&db, page, per_page)` | `Page<Todo>` — [Pagination](pagination)'a bak |

```rust
let user = User::query()
    .filter(User::EMAIL.eq(&email))
    .fetch_one(&db)
    .await?;                       // Option<User>

let already = User::query()
    .filter(User::EMAIL.eq(&email))
    .exists(&db)
    .await?;                       // bool
```

## Pagination

Her row'u yüklemek yerine, `paginate(&db, page, per_page)` bir sayfayı artı bir UI'nin ya da API
istemcisinin ihtiyaç duyduğu toplamları döndürür. Kendi sayfası var: [Pagination](pagination).

## İşlemler (transaction)

Bağlantıda SeaORM'ün işlem API'sini kullan:

```rust
use rustali::orm::sea_orm::TransactionTrait;

db.transaction::<_, (), Error>(|tx| Box::pin(async move {
    some.update(tx).await?;
    other.insert(tx).await?;
    Ok(())
})).await?;
```

Her entity metodu ve `Query` terminali, `&db` aldığı her yerde işlem handle'ını (`tx`) kabul eder.

## SeaORM'e kaçmak

Builder bir şeyi kapsamadığında, `Query::into_select()` doğrudan sürmek için altta yatan SeaORM
`Select<E::Sea>`'ini döndürür ve `rustali::orm::sea_orm` tüm SeaORM API'sini yeniden dışa aktarır.
Bir `DbErr` döndüren ham SeaORM çağrıları `.or_db()?` (`DbResultExt`) ile eşlenebilir; bu, eksik
bir row'u `Error::NotFound`'a ve diğer her şeyi `Error::Database`'e çevirir.

Sıradaki: [Pagination](pagination).
