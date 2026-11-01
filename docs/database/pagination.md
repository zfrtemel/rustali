# Pagination

Loading a whole table into memory does not scale. `paginate` fetches **one page** of results
and, in the same call, the totals a UI or API client needs to render page controls.

## `paginate`

```rust
Todo::query().paginate(&db, page, per_page) -> Result<Page<Todo>>
```

A terminal method on the [query builder](queries): build a query as usual, then finish it with
`paginate` instead of `fetch_all`. `page` is **1-based**. Both `page` and `per_page` are
clamped to at least `1`, so out-of-range values from a query string can never panic or divide
by zero.

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

::: tip Ordering matters
Always give a paginated query a stable `order_by`. Without one the database may return rows in
a different order between pages, so a row can appear twice or be skipped. Order by something
unique and stable, such as the primary key or `created_at` then `id`.
:::

## `Page<T>`

`paginate` returns a `Page<T>` — the rows plus everything needed to render pagination. It
derives `Serialize`, so an API handler can return it directly as JSON.

### Fields

| Field | Meaning |
|---|---|
| `items` | the rows on this page (`Vec<T>`) |
| `page` | the current page, 1-based |
| `per_page` | the page size that was requested |
| `total` | total rows across all pages |
| `last_page` | the number of the last page (always ≥ 1) |

```json
{ "items": [ /* ... */ ], "page": 1, "per_page": 20, "total": 57, "last_page": 3 }
```

### Methods

| Method | Returns |
|---|---|
| `has_next()` | `true` when a next page exists |
| `has_prev()` | `true` when a previous page exists |
| `map(f)` | a `Page<U>` with each item transformed, paging numbers kept |

`has_next` / `has_prev` are what you use to enable or disable "next" and "previous" controls.
`map` turns the rows into a view model without recomputing the totals:

```rust
// entities -> a view model, totals preserved
let page = Todo::query().paginate(&db, 1, 20).await?
    .map(|t| TodoRow { title: t.title, done_label: if t.done { "✓" } else { "" } });
```

## In an API handler

Read the page and size from the query string, cap the size so a client cannot ask for a
million rows, and return the `Page` as JSON:

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

The client gets `{ "items": [...], "page": 1, "per_page": 20, "total": 57, "last_page": 3 }`
and can render controls straight from those numbers.

## In an SSR page

For a server-rendered list, pass the `Page` into the template and drive the controls from its
methods:

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

`fetch_page(&db, page, per_page)` is an older alias of `paginate` with identical behaviour,
kept so it matches the `fetch_*` naming of the other terminal methods. New code should prefer
`paginate`.

Next: [Migrations (Code-First)](migrations).
