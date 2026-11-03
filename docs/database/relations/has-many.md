# has_many

A **one-to-many** relation: one row on this table owns many rows on another. A post has
many comments; an author has many posts. This is the "one" side — the side that does *not*
hold the foreign key.

## Declaring it

```rust
#[entity]
#[table("posts")]
#[has_many(Comment, to = post_id)]
pub struct Post {
    pub id: i64,
    pub title: String,
}

#[entity]
#[table("comments")]
pub struct Comment {
    pub id: i64,
    pub post_id: i64,   // the foreign key lives here
    pub body: String,
}
```

The attribute goes on `Post` (the owner). `to = post_id` names the foreign-key column on
the **target** table (`comments.post_id`) — the column that points back at this row.

### Arguments

| Argument | Meaning | Default |
|---|---|---|
| *(first, positional)* | Target entity type (`Comment`). | — (required) |
| `to = column` | The foreign-key column on the **target** table. | — (required) |
| `from = column` | The local column it points back to. | this table's primary key (`id`) |
| `as = name` | Override the generated method name. | snake-case of the target + `s` (`Comment` → `comments`) |

`to` is required because Rustali does not guess the foreign-key column name. `from`
defaults to the primary key, which is the usual case; set it only when the key is a
different column.

## Generated methods

For `#[has_many(Comment, to = post_id)]`, the derive adds to `Post`:

| Method | Returns | Queries |
|---|---|---|
| `post.comments(&db)` | `Vec<Comment>` | one (lazy) |
| `Post::with_comments(posts, &db)` | `Vec<(Post, Vec<Comment>)>` | one for the batch (eager) |

The method name comes from the target: `Comment` → `comments`. Rename it with
`as`, e.g. `#[has_many(Comment, to = post_id, as = replies)]` gives `post.replies(&db)`
and `Post::with_replies(...)`.

## Lazy — one post's comments

```rust
let post = Post::find_or_404(&db, id).await?;
let comments = post.comments(&db).await?;
println!("{} has {} comments", post.title, comments.len());
```

## Eager — many posts, no N+1

```rust
let posts = Post::all(&db).await?;                  // 1 query
let paired = Post::with_comments(posts, &db).await?; // 1 more query

for (post, comments) in &paired {
    println!("{}: {}", post.title, comments.len());
}
```

Without the eager form, calling `post.comments(&db)` inside the loop would run one query
per post — the N+1 problem. `with_comments` loads every post's comments in a single extra
query and hands them back already grouped.

## The other side

`has_many` is almost always paired with a [`belongs_to`](belongs-to) on the child, so you
can go both ways:

```rust
#[entity]
#[table("comments")]
#[belongs_to(Post, from = post_id)]
pub struct Comment {
    pub id: i64,
    pub post_id: i64,
    pub body: String,
}

let comment = Comment::find_or_404(&db, id).await?;
let post = comment.post(&db).await?;   // Option<Post>
```

Next: [belongs_to](belongs-to).
