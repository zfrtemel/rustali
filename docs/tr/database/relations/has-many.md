# has_many

**Bire-çok** ilişki: bu tablodaki bir satır, başka bir tablodaki birçok satıra sahiptir. Bir
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

Attribute, `Post`'a (sahibe) gelir. `to = post_id`, **hedef** tablodaki yabancı-anahtar kolonunu
(`comments.post_id`) adlandırır — bu satıra geri işaret eden kolon.

### Argümanlar

| Argüman | Anlamı | Varsayılan |
|---|---|---|
| *(ilk, konumsal)* | Hedef varlık tipi (`Comment`). | — (zorunlu) |
| `to = column` | **Hedef** tablodaki yabancı-anahtar kolonu. | — (zorunlu) |
| `from = column` | Geri işaret ettiği yerel kolon. | bu tablonun birincil anahtarı (`id`) |
| `as = name` | Üretilen metot adını geçersiz kıl. | hedefin snake-case'i + `s` (`Comment` → `comments`) |

`to` zorunludur çünkü Rustali yabancı-anahtar kolon adını tahmin etmez. `from`, olağan durum olan
birincil anahtara varsayılan olur; onu yalnızca anahtar farklı bir kolon olduğunda ayarla.

## Üretilen metotlar

`#[has_many(Comment, to = post_id)]` için, derive `Post`'a şunları ekler:

| Metot | Döndürdüğü | Sorgular |
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

İstekli biçim olmadan, döngü içinde `post.comments(&db)`'yi çağırmak gönderi başına bir sorgu
çalıştırırdı — N+1 problemi. `with_comments`, her gönderinin yorumlarını tek bir ek sorguda yükler
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
