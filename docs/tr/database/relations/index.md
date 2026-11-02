# İlişkiler

Tablolar nadiren tek başına durur: bir gönderinin yorumları vardır, bir yorum bir gönderiye aittir,
bir gönderinin birçok etiketi vardır. Rustali, bu bağlantıları varlıkta bir konteyner attribute'u
ile bildirmene izin verir ve ilişkili satırları yüklemek için tipli metotlar üretir.

## Dört tür

| Attribute | Yön | Yüklediği | Sayfa |
|---|---|---|---|
| `#[has_many(...)]` | bir → çok | `Vec<Child>` | [has_many](has-many) |
| `#[belongs_to(...)]` | çok → bir | `Option<Parent>` | [belongs_to](belongs-to) |
| `#[has_one(...)]` | bir → bir | `Option<Related>` | [has_one](has-one) |
| `#[many_to_many(...)]` | çok ↔ çok | `Vec<Other>` (bir bağlantı tablosu üzerinden) | [many_to_many](many-to-many) |

Her biri bir alana değil, **struct**'a yazılır — bir ilişki, saklanan bir kolon değil, tablolar
arasında bir bağlantıdır:

```rust
#[derive(Entity, Clone)]
#[table("posts")]
#[belongs_to(User, from = user_id, as = author)]
#[has_many(Comment, to = post_id)]
#[many_to_many(Tag, via = PostTag, from = post_id, to = tag_id)]
pub struct Post {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
}
```

## Yükleme her zaman açıktır

Rust'ın örtük tembel yüklemesi yoktur. Bir alan erişimi `async` olamaz ve gizli veritabanı G/Ç'si
çalıştıramaz; bu yüzden bir alana dokunduğunda sessizce bir sorgu tetikleyen bir gezinme özelliği
yoktur. Rustali'deki her ilişki yüklemesi, çağırdığın ve `.await`'ladığın bir metottur — bu da
kodda, bir sorgunun tam olarak nerede gerçekleştiğini her zaman görebileceğin anlamına gelir.

İki şekil elde edersin ve bunlar arasında seçim yapmak, N+1 problemini önleme yolundur:

### Tembel — bir ebeveyn, talep üzerine

İlişkiden sonra adlandırılmış bir örnek metodu. İlişkili satırları **o tek satır için** yükler:

```rust
let post = Post::find_or_404(&db, id).await?;
let comments = post.comments(&db).await?;   // Vec<Comment>  (bir sorgu)
let author   = post.author(&db).await?;     // Option<User>  (bir sorgu)
```

Bir kaydın olduğu ve ilişkilerini istediğin bir detay sayfası için mükemmel.

### İstekli (eager) — birçok ebeveyn, gruplanmış (N+1'i önler)

`with_<relation>` adlı bir ilişkili fonksiyon. Zaten getirdiğin ebeveynlerden oluşan bir `Vec`
alır ve **tüm** ilişkili satırlarını tek bir ek sorguda yükler, her ebeveyni kendininkiyle
eşleştirir:

```rust
let posts = Post::all(&db).await?;                   // 1 sorgu
let with_comments = Post::with_comments(posts, &db).await?;
// Vec<(Post, Vec<Comment>)>  — bir sorgu daha, gönderi başına bir tane değil

for (post, comments) in &with_comments {
    println!("{} has {} comments", post.title, comments.len());
}
```

Birçok gönderi üzerindeki bir döngü içinde `post.comments(&db)`'ye başvurmak, klasik N+1 hatasıdır:
liste için bir sorgu, sonra satır başına bir tane daha. `with_comments` bunu toplam iki sorguya
çevirir.

| | Tembel | İstekli |
|---|---|---|
| Çağrı stili | `post.comments(&db)` | `Post::with_comments(posts, &db)` |
| Aldığı | bir satır (`&self`) | birçok satır (`Vec<Self>`) |
| Sorgular | çağrı başına bir | tüm grup için bir |
| Ne zaman kullan | tek bir kaydın olduğunda | bir listen olduğunda ve her birinin ilişkilerine ihtiyacın olduğunda |

## Anahtarlar açıktır

Rustali yabancı-anahtar kolon adlarını tahmin etmez. Onları attribute'ta adlandırırsın (`from` /
`to`); böylece bağlantı görünür olur ve bir yazım hatası, sessiz bir uyuşmazlık değil, bir derleme
hatası olur. Her ilişki sayfası her argümanın tam olarak hangi kolona atıfta bulunduğunu hecelerse
de, kısa sürümü:

- **`belongs_to`** — `from`, **bu** tablodaki yabancı-anahtar kolonudur (zorunlu); `to`, hedefteki
  referans verilen kolondur (varsayılan olarak onun `id`'si).
- **`has_many` / `has_one`** — `to`, **hedef** tablodaki yabancı-anahtar kolonudur (zorunlu);
  `from`, onun geri işaret ettiği yerel kolondur (varsayılan olarak bu tablonun `id`'si).
- **`many_to_many`** — `via`, bağlantı varlığıdır; `from` ve `to`, bağlantının sırasıyla bu tabloya
  ve hedefe işaret eden iki yabancı-anahtar kolonudur.

## Sıradaki nereye

- [has_many](has-many) — birçok çocuğu olan bir ebeveyn.
- [belongs_to](belongs-to) — `has_many`'nin çocuk tarafı.
- [has_one](has-one) — bire-bir bağlantı.
- [many_to_many](many-to-many) — bir bağlantı tablosu üzerinden bağlanan iki tablo.
