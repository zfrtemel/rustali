# many_to_many

A **many-to-many** relation: each row on this table links to many rows on another, and
vice versa. A post has many tags; a tag belongs to many posts. There is no foreign key on
either table — instead a third **junction table** holds one row per link.

## The junction entity

The junction is an ordinary entity with a foreign key to each side. For `Post ↔ Tag` it is
`post_tags`:

```rust
#[derive(Entity, Clone)]
#[table("post_tags")]
#[belongs_to(Post, from = post_id)]
#[belongs_to(Tag, from = tag_id)]
pub struct PostTag {
    pub id: i64,
    pub post_id: i64,
    pub tag_id: i64,
}
```

Declaring the two `belongs_to` relations on the junction is what lets Rustali build the
join. (You can also load junction rows directly, and add extra columns to it — a
`created_at`, a `sort_order` — like any other entity.)

## Declaring the relation

On each side, point at the other through the junction with `#[many_to_many(...)]`:

```rust
#[derive(Entity, Clone)]
#[table("posts")]
#[many_to_many(Tag, via = PostTag, from = post_id, to = tag_id)]
pub struct Post {
    pub id: i64,
    pub title: String,
}

#[derive(Entity, Clone)]
#[table("tags")]
#[many_to_many(Post, via = PostTag, from = tag_id, to = post_id)]
pub struct Tag {
    pub id: i64,
    pub label: String,
}
```

### Arguments

| Argument | Meaning | Default |
|---|---|---|
| *(first, positional)* | Target entity type (`Tag`). | — (required) |
| `via = Entity` | The junction entity (`PostTag`). | — (required) |
| `from = column` | The junction column pointing at **this** entity. | — (required) |
| `to = column` | The junction column pointing at the **target**. | — (required) |
| `as = name` | Override the generated method name. | snake-case of the target + `s` (`Tag` → `tags`) |

Read `from` / `to` from the junction's point of view: on `Post`, `from = post_id` is the
junction column for posts and `to = tag_id` is the junction column for tags. On `Tag` the
two swap, because "this entity" is now the tag.

All three of `via`, `from` and `to` are required — a junction has no primary-key default
the way the one-sided relations do.

## Generated methods

For `#[many_to_many(Tag, via = PostTag, from = post_id, to = tag_id)]`, the derive adds to
`Post`:

| Method | Returns | Queries |
|---|---|---|
| `post.tags(&db)` | `Vec<Tag>` | one (lazy: a single join through the junction) |
| `Post::with_tags(posts, &db)` | `Vec<(Post, Vec<Tag>)>` | two for the batch (eager) |

Because `Tag` declares the mirror relation, the reverse works the same way:
`tag.posts(&db)` and `Tag::with_posts(tags, &db)`.

## Lazy — one post's tags

```rust
let post = Post::find_or_404(&db, id).await?;
let tags = post.tags(&db).await?;   // Vec<Tag>, one query joined through post_tags

// and the other way
let tag = Tag::find_or_404(&db, tag_id).await?;
let posts = tag.posts(&db).await?;  // Vec<Post>
```

## Eager — many posts, no N+1

```rust
let posts = Post::all(&db).await?;               // 1 query
let paired = Post::with_tags(posts, &db).await?; // 2 more queries (junction + tags)

for (post, tags) in &paired {
    let labels: Vec<&str> = tags.iter().map(|t| t.label.as_str()).collect();
    println!("{}: {}", post.title, labels.join(", "));
}
```

The eager loader fetches the junction rows and the target rows in batch, then groups each
post's tags — two queries for the whole list instead of two per post.

## Attaching and detaching links

A link is just a row in the junction table, so you create and remove links by
inserting and deleting junction entities:

```rust
// attach: tag post 1 with tag 2
PostTag { id: 0, post_id: 1, tag_id: 2 }.insert(&db).await?;

// detach: find the link and delete it
if let Some(link) = PostTag::query()
    .filter(PostTag::POST_ID.eq(1))
    .filter(PostTag::TAG_ID.eq(2))
    .fetch_one(&db)
    .await?
{
    link.delete(&db).await?;
}
```

That is all the relations guide covers. Back to [the overview](./).
