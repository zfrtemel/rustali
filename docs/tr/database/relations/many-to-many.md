# many_to_many

Bir **çoka-çok** relation: bu tablodaki her row başka bir tablodaki birçok row'a bağlanır, tersi de
geçerlidir. Bir gönderinin birçok etiketi olur; bir etiket de birçok gönderiye bağlıdır. İki tablonun
hiçbiri yabancı anahtar tutmaz — bunun yerine her bağlantı için bir row barındıran üçüncü bir
**bağlantı tablosu** devreye girer.

## Bağlantı entity'si

Bağlantı tablosu, her iki tarafa da birer yabancı anahtarı olan sıradan bir entity'dir. `Post ↔ Tag`
için bu entity `post_tags`'tir:

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

Bağlantıda iki `belongs_to` relation'ı bildirmen, Rustali'nin join'i kurabilmesini sağlar. (Bağlantı
row'larını doğrudan da yükleyebilir, tıpkı başka bir entity gibi ona ek column'lar ekleyebilirsin —
bir `created_at`, bir `sort_order` gibi.)

## Relation'ı bildirmek

Her iki tarafta da `#[many_to_many(...)]` ile bağlantı üzerinden diğer tabloya işaret et:

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

### Argümanlar

| Argüman | Anlamı | Varsayılan |
|---|---|---|
| *(ilk, konumsal)* | Hedef entity tipi (`Tag`). | — (zorunlu) |
| `via = Entity` | Bağlantı entity'si (`PostTag`). | — (zorunlu) |
| `from = column` | **Bu** entity'ye işaret eden bağlantı column'u. | — (zorunlu) |
| `to = column` | **Hedefe** işaret eden bağlantı column'u. | — (zorunlu) |
| `as = name` | Üretilen metot adını değiştirir. | hedefin snake-case'i + `s` (`Tag` → `tags`) |

`from` / `to`'yu bağlantının bakış açısından düşün: `Post`'ta `from = post_id` gönderiye karşılık gelen
bağlantı column'u, `to = tag_id` ise etikete karşılık gelen bağlantı column'udur. `Tag`'te ikisi yer
değiştirir, çünkü artık "bu entity" etikettir.

`via`, `from` ve `to`'nun üçü de zorunludur — bir bağlantıda, tek taraflı relation'larda olduğu gibi
primary-key'e dayalı bir varsayılan yoktur.

## Üretilen metotlar

`#[many_to_many(Tag, via = PostTag, from = post_id, to = tag_id)]` için, derive `Post`'a şunları
ekler:

| Metot | Döndürdüğü | Query'ler |
|---|---|---|
| `post.tags(&db)` | `Vec<Tag>` | bir (tembel: bağlantı üzerinden tek bir join) |
| `Post::with_tags(posts, &db)` | `Vec<(Post, Vec<Tag>)>` | grup için iki (istekli) |

`Tag` da aynadaki relation'ı bildirdiği için ters yön de birebir aynı şekilde çalışır: `tag.posts(&db)`
ve `Tag::with_posts(tags, &db)`.

## Tembel — bir gönderinin etiketleri

```rust
let post = Post::find_or_404(&db, id).await?;
let tags = post.tags(&db).await?;   // Vec<Tag>, post_tags aracılığıyla join'lenmiş bir sorgu

// ve diğer yön
let tag = Tag::find_or_404(&db, tag_id).await?;
let posts = tag.posts(&db).await?;  // Vec<Post>
```

## İstekli — birçok gönderi, N+1 yok

```rust
let posts = Post::all(&db).await?;               // 1 sorgu
let paired = Post::with_tags(posts, &db).await?; // 2 sorgu daha (bağlantı + etiketler)

for (post, tags) in &paired {
    let labels: Vec<&str> = tags.iter().map(|t| t.label.as_str()).collect();
    println!("{}: {}", post.title, labels.join(", "));
}
```

İstekli yükleyici önce bağlantı row'larını ve hedef row'larını topluca getirir, ardından her
gönderinin etiketlerini gruplar — yani gönderi başına iki query yerine tüm liste için toplam iki
query.

## Bağlantıları iliştirmek ve ayırmak

Bir bağlantı, bağlantı tablosundaki bir row'dan ibarettir; dolayısıyla bağlantıları, bağlantı
entity'lerini ekleyip silerek oluşturur ve kaldırırsın:

```rust
// iliştir: gönderi 1'i etiket 2 ile etiketle
PostTag { id: 0, post_id: 1, tag_id: 2 }.insert(&db).await?;

// ayır: bağlantıyı bul ve sil
if let Some(link) = PostTag::query()
    .filter(PostTag::POST_ID.eq(1))
    .filter(PostTag::TAG_ID.eq(2))
    .fetch_one(&db)
    .await?
{
    link.delete(&db).await?;
}
```

Relation'lar rehberinin kapsadığı konular bu kadar. [Genel bakışa](./) dönebilirsin.
