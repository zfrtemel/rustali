# belongs_to

Bire-çok relation'ın **çocuk tarafı**: bu row, tuttuğu bir yabancı anahtar aracılığıyla başka bir
tablodaki bir row'a işaret eder. Bir yorum bir gönderiye aittir; bir gönderi bir yazara aittir. Bu,
yabancı-anahtar column'unu *tutan* taraftır.

## Bildirmek

```rust
#[derive(Entity, Clone)]
#[table("comments")]
#[belongs_to(Post, from = post_id)]
pub struct Comment {
    pub id: i64,
    pub post_id: i64,   // yabancı anahtar burada, bu tabloda yaşar
    pub body: String,
}
```

Attribute, `Comment`'e (anahtara sahip olan çocuk) gelir. `from = post_id`, **bu** tablodaki
yabancı-anahtar column'unu adlandırır.

### Argümanlar

| Argüman | Anlamı | Varsayılan |
|---|---|---|
| *(ilk, konumsal)* | Hedef entity tipi (`Post`). | — (zorunlu) |
| `from = column` | **Bu** tablodaki yabancı-anahtar column'u. | — (zorunlu) |
| `to = column` | Hedefteki referans verilen column. | hedefin primary key'i (`id`) |
| `as = name` | Üretilen metot adını geçersiz kıl. | hedefin snake-case'i (`post`) |

`from` zorunludur. `to`, yabancı anahtarın hedefin primary key'ine referans verdiği normal
durumu kapsayan hedefin `id`'sine varsayılan olur; onu yalnızca farklı bir column'a referans vermek
için ayarla.

## Üretilen metotlar

`#[belongs_to(Post, from = post_id)]` için, derive `Comment`'e şunları ekler:

| Metot | Döndürdüğü | Query'ler |
|---|---|---|
| `comment.post(&db)` | `Option<Post>` | bir (tembel) |
| `Comment::with_post(comments, &db)` | `Vec<(Comment, Option<Post>)>` | grup için bir (istekli) |

Sonuç bir `Option`'dır çünkü yabancı anahtar artık var olmayan bir row'a referans verebilir (ya da
null olabilir). Metot adı hedeften gelir: `Post` → `post`.

## Metodu `as` ile adlandırmak

Relation farklı bir ad altında daha iyi okunduğunda — ya da bir tablonun aynı hedefe iki yabancı
anahtarı olduğunda — `as` kullan:

```rust
#[derive(Entity, Clone)]
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

## Tembel ve istekli

```rust
// Tembel — bir yorumun gönderisi
let comment = Comment::find_or_404(&db, id).await?;
let post = comment.post(&db).await?;

// İstekli — birçok yorum, gönderileri tek bir ek sorguda
let comments = Comment::all(&db).await?;
let paired = Comment::with_post(comments, &db).await?; // Vec<(Comment, Option<Post>)>
for (comment, post) in &paired {
    if let Some(post) = post {
        println!("{} -> {}", comment.body, post.title);
    }
}
```

## has_many ile eşleşir

Çocuktaki `belongs_to` ve ebeveyndeki [`has_many`](has-many), aynı bağlantıyı iki uçtan tanımlar.
Her iki yönde de gezinmek için ikisini de bildir:

```rust
// ebeveyn
#[has_many(Comment, to = post_id)]
struct Post { /* ... */ }

// çocuk
#[belongs_to(Post, from = post_id)]
struct Comment { /* ... */ }
```

Sıradaki: [has_one](has-one).
