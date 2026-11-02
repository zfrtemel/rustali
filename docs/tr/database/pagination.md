# Sayfalama

Tüm bir tabloyu belleğe yüklemek ölçeklenmez. `paginate`, sonuçların **bir sayfasını** ve aynı
çağrıda, bir UI'nin ya da API istemcisinin sayfa kontrollerini render etmek için ihtiyaç duyduğu
toplamları getirir.

## `paginate`

```rust
Todo::query().paginate(&db, page, per_page) -> Result<Page<Todo>>
```

[Sorgu builder'ı](queries) üzerinde bir terminal metot: her zamanki gibi bir sorgu kur, sonra onu
`fetch_all` yerine `paginate` ile bitir. `page` **1 tabanlıdır**. Hem `page` hem `per_page` en az
`1`'e sıkıştırılır; böylece bir sorgu dizesinden gelen aralık dışı değerler asla panikleyemez ya
da sıfıra bölemez.

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
Sayfalanmış bir sorguya her zaman kararlı bir `order_by` ver. Biri olmadan, veritabanı sayfalar
arasında satırları farklı bir sırada döndürebilir; böylece bir satır iki kez görünebilir ya da
atlanabilir. Birincil anahtar ya da `created_at` sonra `id` gibi benzersiz ve kararlı bir şeye göre
sırala.
:::

## `Page<T>`

`paginate`, bir `Page<T>` döndürür — satırlar artı sayfalamayı render etmek için gereken her şey.
`Serialize`'ı türetir; böylece bir API handler'ı onu doğrudan JSON olarak döndürebilir.

### Alanlar

| Alan | Anlamı |
|---|---|
| `items` | bu sayfadaki satırlar (`Vec<T>`) |
| `page` | geçerli sayfa, 1 tabanlı |
| `per_page` | istenen sayfa boyutu |
| `total` | tüm sayfalardaki toplam satır |
| `last_page` | son sayfanın numarası (her zaman ≥ 1) |

```json
{ "items": [ /* ... */ ], "page": 1, "per_page": 20, "total": 57, "last_page": 3 }
```

### Metotlar

| Metot | Döndürdüğü |
|---|---|
| `has_next()` | bir sonraki sayfa varsa `true` |
| `has_prev()` | bir önceki sayfa varsa `true` |
| `map(f)` | her öğesi dönüştürülmüş, sayfalama sayıları korunmuş bir `Page<U>` |

`has_next` / `has_prev`, "sonraki" ve "önceki" kontrollerini etkinleştirmek ya da devre dışı
bırakmak için kullandığın şeydir. `map`, toplamları yeniden hesaplamadan satırları bir görünüm
modeline çevirir:

```rust
// varlıklar -> bir görünüm modeli, toplamlar korunmuş
let page = Todo::query().paginate(&db, 1, 20).await?
    .map(|t| TodoRow { title: t.title, done_label: if t.done { "✓" } else { "" } });
```

## Bir API handler'ında

Sayfayı ve boyutu sorgu dizesinden oku, bir istemcinin bir milyon satır isteyememesi için boyutu
sınırla ve `Page`'i JSON olarak döndür:

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

İstemci `{ "items": [...], "page": 1, "per_page": 20, "total": 57, "last_page": 3 }` alır ve
kontrolleri doğrudan bu sayılardan render edebilir.

## Bir SSR sayfasında

Sunucuda render edilen bir liste için, `Page`'i şablona geçir ve kontrolleri metotlarından yönlendir:

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

`fetch_page(&db, page, per_page)`, `paginate`'in aynı davranışa sahip eski bir takma adıdır; diğer
terminal metotların `fetch_*` adlandırmasıyla eşleşsin diye tutulur. Yeni kod `paginate`'i tercih
etmelidir.

Sıradaki: [Migration'lar (Code-First)](migrations).
