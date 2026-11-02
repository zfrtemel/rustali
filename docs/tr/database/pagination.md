# Pagination

Bütün bir tabloyu belleğe yüklemek ölçeklenebilir bir çözüm değildir. `paginate`, sonuçların yalnızca
**bir sayfasını** getirir ve aynı çağrıda, bir UI'nin ya da API istemcisinin sayfa kontrollerini
render etmek için ihtiyaç duyduğu toplam sayıları da verir.

## `paginate`

```rust
Todo::query().paginate(&db, page, per_page) -> Result<Page<Todo>>
```

[Query builder'ı](queries) üzerinde bir terminal metottur: query'yi her zamanki gibi kur, sonra
`fetch_all` yerine `paginate` ile bitir. `page` değeri **1'den başlar**. Hem `page` hem `per_page` en
az `1` olacak şekilde kırpılır; böylece bir query string'inden gelen aralık dışı değerler ne panik
oluşturabilir ne de sıfıra bölmeye yol açabilir.

```rust
let page = Todo::query()
    .order_by_desc(Todo::CREATED_AT)
    .paginate(&db, 1, 20)
    .await?;

println!("page {} of {}", page.page, page.last_page);
for todo in &page.items {
    println!("{}", todo.title);
}
```

::: tip Sıralama önemlidir
Sayfalanan bir query'ye her zaman kararlı bir `order_by` ver. Sıralama olmadan veritabanı, row'ları
sayfadan sayfaya farklı bir sırayla döndürebilir; bu durumda bir row ya iki kez görünür ya da büsbütün
atlanır. Primary key gibi, ya da `created_at` ardından `id` gibi benzersiz ve kararlı bir ölçüte göre
sırala.
:::

## `Page<T>`

`paginate` bir `Page<T>` döndürür — yani row'ları ve pagination'ı render etmek için gereken her şeyi.
`Serialize`'ı türetir; böylece bir API handler'ı onu doğrudan JSON olarak döndürebilir.

### Alanlar

| Alan | Anlamı |
|---|---|
| `items` | bu sayfadaki row'lar (`Vec<T>`) |
| `page` | geçerli sayfa (1'den başlar) |
| `per_page` | istenen sayfa boyutu |
| `total` | tüm sayfalardaki toplam row sayısı |
| `last_page` | son sayfanın numarası (her zaman ≥ 1) |

```json
{ "items": [ /* ... */ ], "page": 1, "per_page": 20, "total": 57, "last_page": 3 }
```

### Metotlar

| Metot | Döndürdüğü |
|---|---|
| `has_next()` | sonraki sayfa varsa `true` |
| `has_prev()` | önceki sayfa varsa `true` |
| `map(f)` | her öğesi dönüştürülmüş, pagination sayıları korunmuş bir `Page<U>` |

`has_next` / `has_prev`, "sonraki" ve "önceki" kontrollerini etkinleştirmek ya da devre dışı
bırakmak için kullandığın metotlardır. `map` ise toplamları yeniden hesaplamadan row'ları bir view
modeline dönüştürür:

```rust
// varlıklar -> bir görünüm modeli, toplamlar korunmuş
let page = Todo::query().paginate(&db, 1, 20).await?
    .map(|t| TodoRow { title: t.title, done_label: if t.done { "✓" } else { "" } });
```

## Bir API handler'ında

Sayfa numarasını ve sayfa boyutunu query string'inden oku, bir istemcinin bir milyon row
isteyememesi için boyuta bir üst sınır koy ve `Page`'i JSON olarak döndür:

```rust
#[derive(Deserialize)]
struct Paging { page: Option<u64>, per_page: Option<u64> }

#[get("/")]
async fn index(db: Db, Query(p): Query<Paging>) -> Result<Json<Page<Todo>>> {
    let page = Todo::query()
        .order_by_desc(Todo::ID)
        .paginate(&db, p.page.unwrap_or(1), p.per_page.unwrap_or(20).min(100))
        .await?;
    Ok(Json(page))
}
```

İstemci `{ "items": [...], "page": 1, "per_page": 20, "total": 57, "last_page": 3 }` yanıtını alır ve
kontrolleri doğrudan bu sayılara bakarak render edebilir.

## Bir SSR sayfasında

Sunucuda render edilen bir liste için `Page`'i template'e geçir ve kontrolleri onun metotlarıyla
yönet:

```rust
#[get("/")]
async fn index(db: Db, Query(p): Query<Paging>) -> Result<View<TodoList>> {
    let page = Todo::query().order_by_desc(Todo::ID)
        .paginate(&db, p.page.unwrap_or(1), 20).await?;
    Ok(View::new(TodoList { page }))
}
```

```html
{% for todo in page.items %}
  <li>{{ todo.title }}</li>
{% endfor %}

{% if page.has_prev() %}<a href="?page={{ page.page - 1 }}">Previous</a>{% endif %}
{% if page.has_next() %}<a href="?page={{ page.page + 1 }}">Next</a>{% endif %}
```

## `fetch_page`

`fetch_page(&db, page, per_page)`, `paginate` ile birebir aynı davranan eski bir takma addır; diğer
terminal metotlardaki `fetch_*` adlandırmasına uysun diye korunur. Yeni kodda `paginate` tercih
edilmelidir.

Sıradaki: [Migration'lar (Code-First)](migrations).
