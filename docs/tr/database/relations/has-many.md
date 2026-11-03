# has_many

**Bire-çok** relation: bu tablodaki bir row, başka bir tablodaki birçok row'a sahiptir. Bir gönderinin
birçok yorumu olur; bir yazarın birçok gönderisi olur. Burada söz konusu olan "bir" tarafıdır — yani
yabancı anahtarı *tutmayan* taraf.

## Bildirmek

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
    pub post_id: i64,   // yabancı anahtar burada yaşar
    pub body: String,
}
```

Attribute, sahip olan tarafa, yani `Post`'a yazılır. `to = post_id`, **hedef** tablodaki
yabancı-anahtar column'unu (`comments.post_id`) belirtir — bu row'a geri işaret eden column'u.

### Argümanlar

| Argüman | Anlamı | Varsayılan |
|---|---|---|
| *(ilk, konumsal)* | Hedef entity tipi (`Comment`). | — (zorunlu) |
| `to = column` | **Hedef** tablodaki yabancı-anahtar column'u. | — (zorunlu) |
| `from = column` | Bunun geri işaret ettiği yerel column. | bu tablonun primary key'i (`id`) |
| `as = name` | Üretilen metot adını değiştirir. | hedefin snake-case'i + `s` (`Comment` → `comments`) |

`to` zorunludur, çünkü Rustali yabancı-anahtar column'unun adını tahmin etmez. `from` ise olağan
durum olan primary key'e varsayılır; onu yalnızca anahtar farklı bir column olduğunda belirtmen
gerekir.

## Üretilen metotlar

`#[has_many(Comment, to = post_id)]` için, derive `Post`'a şunları ekler:

| Metot | Döndürdüğü | Query'ler |
|---|---|---|
| `post.comments(&db)` | `Vec<Comment>` | bir (tembel) |
| `Post::with_comments(posts, &db)` | `Vec<(Post, Vec<Comment>)>` | grup için bir (istekli) |

Metot adı hedeften türetilir: `Comment` → `comments`. `as` ile yeniden adlandırabilirsin; örneğin
`#[has_many(Comment, to = post_id, as = replies)]`, sana `post.replies(&db)` ve
`Post::with_replies(...)` metotlarını verir.

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

İstekli biçimi kullanmadan döngü içinde `post.comments(&db)` çağırsaydın, her gönderi için ayrı bir
query çalışırdı — işte N+1 problemi budur. `with_comments` ise her gönderinin yorumlarını tek bir ek
query'de yükler ve onları önceden gruplanmış hâlde geri verir.

## Diğer taraf

`has_many`, neredeyse her zaman çocuk tarafındaki bir [`belongs_to`](belongs-to) ile birlikte
kullanılır; böylece bağlantıda her iki yöne de gidebilirsin:

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

Sıradaki: [belongs_to](belongs-to).
