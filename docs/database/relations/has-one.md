# has_one

A **one-to-one** relation from the owning side: this row has exactly one related row on
another table, and that other table holds the foreign key. An author has one profile; a
user has one set of settings.

`has_one` is the one-row twin of [`has_many`](has-many): same owning side, same foreign-key
placement, but the related table has at most one matching row, so you get an `Option`
instead of a `Vec`.

## Declaring it

```rust
#[derive(Entity, Clone)]
#[table("authors")]
#[has_one(Profile, to = author_id)]
pub struct Author {
    pub id: i64,
    pub name: String,
}

#[derive(Entity, Clone)]
#[table("profiles")]
#[belongs_to(Author, from = author_id)]
pub struct Profile {
    pub id: i64,
    pub author_id: i64,   // the foreign key lives here
    pub bio: String,
}
```

The attribute goes on `Author` (the owner). `to = author_id` names the foreign-key column
on the **target** table (`profiles.author_id`).

### Arguments

| Argument | Meaning | Default |
|---|---|---|
| *(first, positional)* | Target entity type (`Profile`). | — (required) |
| `to = column` | The foreign-key column on the **target** table. | — (required) |
| `from = column` | The local column it points back to. | this table's primary key (`id`) |
| `as = name` | Override the generated method name. | snake-case of the target (`profile`) |

Same arguments as `has_many` — only the cardinality differs.

## Generated methods

For `#[has_one(Profile, to = author_id)]`, the derive adds to `Author`:

| Method | Returns | Queries |
|---|---|---|
| `author.profile(&db)` | `Option<Profile>` | one (lazy) |
| `Author::with_profile(authors, &db)` | `Vec<(Author, Option<Profile>)>` | one for the batch (eager) |

## Lazy and eager

```rust
// Lazy — one author's profile
let author = Author::find_or_404(&db, id).await?;
let profile = author.profile(&db).await?;   // Option<Profile>

// Eager — many authors, their profiles in one extra query
let authors = Author::all(&db).await?;
let paired = Author::with_profile(authors, &db).await?;
for (author, profile) in &paired {
    if let Some(p) = profile {
        println!("{}: {}", author.name, p.bio);
    }
}
```

## has_one vs belongs_to

Both return `Option<T>`, so which do you use? It comes down to **which table holds the
foreign key**:

- The table *without* the key uses `has_one` (here, `Author`).
- The table *with* the key uses [`belongs_to`](belongs-to) (here, `Profile`, which has
  `author_id`).

Declare both to navigate in either direction, exactly as with `has_many` / `belongs_to`.

Next: [many_to_many](many-to-many).
