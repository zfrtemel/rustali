# Relationships

Tables rarely stand alone: a post has comments, a comment belongs to a post, a post has
many tags. Rustali lets you declare these links on the entity with a container attribute,
and generates typed methods to load the related rows.

## The four kinds

| Attribute | Direction | Loads | Page |
|---|---|---|---|
| `#[has_many(...)]` | one → many | `Vec<Child>` | [has_many](has-many) |
| `#[belongs_to(...)]` | many → one | `Option<Parent>` | [belongs_to](belongs-to) |
| `#[has_one(...)]` | one → one | `Option<Related>` | [has_one](has-one) |
| `#[many_to_many(...)]` | many ↔ many | `Vec<Other>` (through a junction) | [many_to_many](many-to-many) |

Each one is written on the **struct**, not a field — a relation is a link between tables,
not a stored column:

```rust
#[derive(Entity, Clone)]
#[table("posts")]
#[belongs_to(User, from = user_id, as = author)]
#[has_many(Comment, to = post_id)]
#[many_to_many(Tag, via = PostTag, from = post_id, to = tag_id)]
pub struct Post {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
}
```

## Loading is always explicit

Rust has no implicit lazy loading. A field access cannot be `async` and cannot run hidden
database I/O, so there is no navigation property that quietly fires a query when you touch a
field. Every relation load in Rustali is a method you call and `.await` — which means you can
always see, in the code, exactly where a query happens.

You get two shapes, and choosing between them is how you avoid the N+1 problem:

### Lazy — one parent, on demand

An instance method named after the relation. Loads the related rows for **that one row**:

```rust
let post = Post::find_or_404(&db, id).await?;
let comments = post.comments(&db).await?;   // Vec<Comment>  (one query)
let author   = post.author(&db).await?;     // Option<User>  (one query)
```

Perfect for a detail page where you have one record and want its relations.

### Eager — many parents, batched (avoids N+1)

An associated function named `with_<relation>`. It takes a `Vec` of parents you already
fetched and loads **all** their related rows in one extra query, pairing each parent with
its own:

```rust
let posts = Post::all(&db).await?;                   // 1 query
let with_comments = Post::with_comments(posts, &db).await?;
// Vec<(Post, Vec<Comment>)>  — one more query, not one per post

for (post, comments) in &with_comments {
    println!("{} has {} comments", post.title, comments.len());
}
```

Reaching for `post.comments(&db)` inside a loop over many posts is the classic N+1
mistake: one query for the list, then one more per row. `with_comments` turns that into
two queries total.

| | Lazy | Eager |
|---|---|---|
| Call style | `post.comments(&db)` | `Post::with_comments(posts, &db)` |
| Takes | one row (`&self`) | many rows (`Vec<Self>`) |
| Queries | one per call | one for the whole batch |
| Use when | you have a single record | you have a list and need each one's relations |

## Keys are explicit

Rustali does not guess foreign-key column names. You name them in the attribute (`from` /
`to`), so the link is visible and a typo is a compile error rather than a silent
mismatch. Each relation page spells out exactly which column each argument refers to, but
the short version:

- **`belongs_to`** — `from` is the foreign-key column on **this** table (required); `to` is
  the referenced column on the target (defaults to its `id`).
- **`has_many` / `has_one`** — `to` is the foreign-key column on the **target** table
  (required); `from` is the local column it points back to (defaults to this table's `id`).
- **`many_to_many`** — `via` is the junction entity; `from` and `to` are the junction's two
  foreign-key columns, pointing at this table and the target respectively.

## Where to go next

- [has_many](has-many) — a parent with many children.
- [belongs_to](belongs-to) — the child side of `has_many`.
- [has_one](has-one) — a one-to-one link.
- [many_to_many](many-to-many) — two tables linked through a junction.
