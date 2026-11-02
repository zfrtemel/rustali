# many_to_many

Bir **çoka-çok** ilişki: bu tablodaki her satır başka bir tablodaki birçok satıra bağlanır ve tersi.
Bir gönderinin birçok etiketi vardır; bir etiket birçok gönderiye aittir. Her iki tabloda da yabancı
anahtar yoktur — bunun yerine üçüncü bir **bağlantı tablosu** bağlantı başına bir satır tutar.

## Bağlantı varlığı

Bağlantı, her iki tarafa da bir yabancı anahtarı olan sıradan bir varlıktır. `Post ↔ Tag` için
`post_tags`'tir:

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

İki `belongs_to` ilişkisini bağlantıda bildirmek, Rustali'nin join'i kurmasını sağlayan şeydir.
(Bağlantı satırlarını doğrudan da yükleyebilir ve ona ekstra kolonlar ekleyebilirsin — bir
`created_at`, bir `sort_order` — diğer herhangi bir varlık gibi.)

## İlişkiyi bildirmek

Her tarafta, `#[many_to_many(...)]` ile bağlantı aracılığıyla diğerine işaret et:

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
| *(ilk, konumsal)* | Hedef varlık tipi (`Tag`). | — (zorunlu) |
| `via = Entity` | Bağlantı varlığı (`PostTag`). | — (zorunlu) |
| `from = column` | **Bu** varlığa işaret eden bağlantı kolonu. | — (zorunlu) |
| `to = column` | **Hedefe** işaret eden bağlantı kolonu. | — (zorunlu) |
| `as = name` | Üretilen metot adını geçersiz kıl. | hedefin snake-case'i + `s` (`Tag` → `tags`) |

`from` / `to`'yu bağlantının bakış açısından oku: `Post`'ta, `from = post_id` gönderiler için
bağlantı kolonudur ve `to = tag_id` etiketler için bağlantı kolonudur. `Tag`'te ikisi yer değiştirir,
çünkü "bu varlık" artık etikettir.

`via`, `from` ve `to`'nun üçü de zorunludur — bir bağlantının, tek taraflı ilişkilerin olduğu gibi
bir birincil-anahtar varsayılanı yoktur.

## Üretilen metotlar

`#[many_to_many(Tag, via = PostTag, from = post_id, to = tag_id)]` için, derive `Post`'a şunları
ekler:

| Metot | Döndürdüğü | Sorgular |
|---|---|---|
| `post.tags(&db)` | `Vec<Tag>` | bir (tembel: bağlantı aracılığıyla tek bir join) |
| `Post::with_tags(posts, &db)` | `Vec<(Post, Vec<Tag>)>` | grup için iki (istekli) |

`Tag`, ayna ilişkisini bildirdiği için, ters yön de aynı şekilde çalışır: `tag.posts(&db)` ve
`Tag::with_posts(tags, &db)`.

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

İstekli yükleyici, bağlantı satırlarını ve hedef satırlarını toplu olarak getirir, sonra her
gönderinin etiketlerini gruplar — gönderi başına iki yerine tüm liste için iki sorgu.

## Bağlantıları iliştirmek ve ayırmak

Bir bağlantı yalnızca bağlantı tablosundaki bir satırdır; bu yüzden bağlantı varlıklarını ekleyerek
ve silerek bağlantıları oluşturur ve kaldırırsın:

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

İlişkiler rehberinin kapsadığı her şey bu kadar. [Genel bakışa](./) dön.
