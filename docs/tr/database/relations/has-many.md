# has_many

**Bire-çok** relation: bu tablodaki bir row, başka bir tablodaki birçok row'a sahiptir. Bir
gönderinin birçok yorumu vardır; bir yazarın birçok gönderisi vardır. Bu "bir" tarafıdır — yabancı
anahtarı *tutmayan* taraf.

## Bildirmek

```rust
#[derive(Entity, Clone)]
#[table("posts")]
#[has_many(Comment, to = post_id)]
pub struct Post {
    pub id: i64,
    pub title: String,
}

#[derive(Entity, Clone)]
#[table("comments")]
pub struct Comment {
    pub id: i64,
    pub post_id: i64,   // yabancı anahtar burada yaşar
    pub body: String,
}
```

Attribute, `Post`'a (sahibe) gelir. `to = post_id`, **hedef** tablodaki yabancı-anahtar column'unu
(`comments.post_id`) adlandırır — bu row'a geri işaret eden column.

### Argümanlar

| Argüman | Anlamı | Varsayılan |
|---|---|---|
| *(ilk, konumsal)* | Hedef entity tipi (`Comment`). | — (zorunlu) |
| `to = column` | **Hedef** tablodaki yabancı-anahtar column'u. | — (zorunlu) |
| `from = column` | Geri işaret ettiği yerel column. | bu tablonun primary key'i (`id`) |
| `as = name` | Üretilen metot adını geçersiz kıl. | hedefin snake-case'i + `s` (`Comment` → `comments`) |

`to` zorunludur çünkü Rustali yabancı-anahtar column adını tahmin etmez. `from`, olağan durum olan
primary key'e varsayılan olur; onu yalnızca anahtar farklı bir column olduğunda ayarla.

## Üretilen metotlar

`#[has_many(Comment, to = post_id)]` için, derive `Post`'a şunları ekler:

| Metot | Döndürdüğü | Query'ler |
|---|---|---|
| `post.comments(&db)` | `Vec<Comment>` | bir (tembel) |
| `Post::with_comments(posts, &db)` | `Vec<(Post, Vec<Comment>)>` | grup için bir (istekli) |

Metot adı hedeften gelir: `Comment` → `comments`. Onu `as` ile yeniden adlandır, örn.
`#[has_many(Comment, to = post_id, as = replies)]` sana `post.replies(&db)` ve
`Post::with_replies(...)` verir.

## Tembel — bir gönderinin yorumları

```rust
let post = Post::find_or_404(&db, id).await?;
let comments = post.comments(&db).await?;
println!("{} has {} comments", post.title, comments.len());
```

## İstekli — birçok gönderi, N+1 yok

```rust
let posts = Post::all(&db).await?;                  // 1 sorgu
let paired = Post::with_comments(posts, &db).await?; // 1 sorgu daha

for (post, comments) in &paired {
    println!("{}: {}", post.title, comments.len());
}
```

İstekli biçim olmadan, döngü içinde `post.comments(&db)`'yi çağırmak gönderi başına bir query
çalıştırırdı — N+1 problemi. `with_comments`, her gönderinin yorumlarını tek bir ek query'de yükler
ve onları önceden gruplanmış olarak geri verir.

## Diğer taraf

`has_many`, neredeyse her zaman çocukta bir [`belongs_to`](belongs-to) ile eşleştirilir; böylece
her iki yöne de gidebilirsin:

```rust
#[derive(Entity, Clone)]
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

Sıradaki: [belongs_to](belongs-to).
