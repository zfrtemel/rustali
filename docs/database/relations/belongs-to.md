# belongs_to

The **child side** of a one-to-many relation: this row points at one row on another table
through a foreign key it holds. A comment belongs to a post; a post belongs to an author.
This is the side that *holds* the foreign-key column.

## Declaring it

```rust
#[entity]
#[table("comments")]
#[belongs_to(Post, from = post_id)]
pub struct Comment {
    pub id: i64,
    pub post_id: i64,   // the foreign key lives here, on this table
    pub body: String,
}
```

The attribute goes on `Comment` (the child, which owns the key). `from = post_id` names the
foreign-key column on **this** table.

### Arguments

| Argument | Meaning | Default |
|---|---|---|
| *(first, positional)* | Target entity type (`Post`). | — (required) |
| `from = column` | The foreign-key column on **this** table. | — (required) |
| `to = column` | The referenced column on the target. | target's primary key (`id`) |
| `as = name` | Override the generated method name. | snake-case of the target (`post`) |

`from` is required. `to` defaults to the target's `id`, which covers the normal case where
the foreign key references the target's primary key; set it only to reference a different
column.

## Generated methods

For `#[belongs_to(Post, from = post_id)]`, the derive adds to `Comment`:

| Method | Returns | Queries |
|---|---|---|
| `comment.post(&db)` | `Option<Post>` | one (lazy) |
| `Comment::with_post(comments, &db)` | `Vec<(Comment, Option<Post>)>` | one for the batch (eager) |

The result is an `Option` because the foreign key could reference a row that no longer
exists (or be nullable). The method name comes from the target: `Post` → `post`.

## Naming the method with `as`

When the relation reads better under a different name — or when a table has two foreign
keys to the same target — use `as`:

```rust
#[entity]
#[table("posts")]
#[belongs_to(User, from = user_id, as = author)]
pub struct Post {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
}

let post = Post::find_or_404(&db, id).await?;
let author = post.author(&db).await?;   // Option<User>
```

## Lazy and eager

```rust
// Lazy — one comment's post
let comment = Comment::find_or_404(&db, id).await?;
let post = comment.post(&db).await?;

// Eager — many comments, their posts in one extra query
let comments = Comment::all(&db).await?;
let paired = Comment::with_post(comments, &db).await?; // Vec<(Comment, Option<Post>)>
for (comment, post) in &paired {
    if let Some(post) = post {
        println!("{} -> {}", comment.body, post.title);
    }
}
```

## Pairs with has_many

`belongs_to` on the child and [`has_many`](has-many) on the parent describe the same link
from the two ends. Declare both to navigate in either direction:

```rust
// parent
#[has_many(Comment, to = post_id)]
struct Post { /* ... */ }

// child
#[belongs_to(Post, from = post_id)]
struct Comment { /* ... */ }
```

Next: [has_one](has-one).
