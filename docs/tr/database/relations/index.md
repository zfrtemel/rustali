# Relation'lar

Tablolar nadiren tek başına durur: bir gönderinin yorumları vardır, bir yorum bir gönderiye aittir,
bir gönderinin birçok etiketi olabilir. Rustali, bu bağlantıları entity üzerinde bir konteyner
attribute'uyla bildirmene olanak tanır ve ilişkili row'ları yüklemen için tipli metotlar üretir.

## Dört tür

| Attribute | Yön | Yüklediği | Sayfa |
|---|---|---|---|
| `#[has_many(...)]` | bir → çok | `Vec<Child>` | [has_many](has-many) |
| `#[belongs_to(...)]` | çok → bir | `Option<Parent>` | [belongs_to](belongs-to) |
| `#[has_one(...)]` | bir → bir | `Option<Related>` | [has_one](has-one) |
| `#[many_to_many(...)]` | çok ↔ çok | `Vec<Other>` (bir bağlantı tablosu üzerinden) | [many_to_many](many-to-many) |

Bunların her biri bir alana değil, doğrudan **struct**'a yazılır — çünkü bir relation, saklanan bir
column değil, tablolar arasındaki bir bağlantıdır:

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

Rust'ta örtük tembel yükleme yoktur. Bir alan erişimi `async` olamaz ve arka planda veritabanı G/Ç'si
yapamaz; bu yüzden bir alana dokunduğunda sessizce bir query tetikleyen "gezinme özelliği" diye bir
şey yoktur. Rustali'de her relation yüklemesi, senin çağırıp `.await`'lediğin bir metottur — bu da bir
query'nin kodda tam olarak nerede çalıştığını her zaman görebilmen demektir.

İki farklı biçimle karşılaşırsın; bunlar arasında seçim yapmak da N+1 problemini önlemenin yoludur:

### Tembel — bir ebeveyn, talep üzerine

Adını relation'dan alan bir örnek metodu. İlişkili row'ları **yalnızca o tek row için** yükler:

```rust
let post = Post::find_or_404(&db, id).await?;
let comments = post.comments(&db).await?;   // Vec<Comment>  (bir sorgu)
let author   = post.author(&db).await?;     // Option<User>  (bir sorgu)
```

Elinde tek bir kaydın olduğu ve onun ilişkilerini görmek istediğin bir detay sayfası için birebir.

### İstekli (eager) — birçok ebeveyn, gruplanmış (N+1'i önler)

`with_<relation>` adında ilişkili bir fonksiyon. Daha önce getirdiğin ebeveynlerden oluşan bir `Vec`
alır, hepsinin **tüm** ilişkili row'larını tek bir ek query'de yükler ve her ebeveyni kendi row'larıyla
eşleştirir:

```rust
let posts = Post::all(&db).await?;                   // 1 sorgu
let with_comments = Post::with_comments(posts, &db).await?;
// Vec<(Post, Vec<Comment>)>  — bir sorgu daha, gönderi başına bir tane değil

for (post, comments) in &with_comments {
    println!("{} has {} comments", post.title, comments.len());
}
```

Çok sayıda gönderi üzerinde dönen bir döngü içinde `post.comments(&db)` çağırmak, klasik N+1 hatasıdır:
liste için bir query, sonra her row için bir tane daha. `with_comments` bunu toplam iki query'ye
indirir.

| | Tembel | İstekli |
|---|---|---|
| Çağrı stili | `post.comments(&db)` | `Post::with_comments(posts, &db)` |
| Aldığı | tek bir row (`&self`) | birçok row (`Vec<Self>`) |
| Query sayısı | çağrı başına bir | tüm grup için bir |
| Ne zaman kullanılır | elinde tek bir kayıt varken | elinde bir liste varken ve her birinin relation'larına ihtiyacın varken |

## Anahtarlar açıktır

Rustali yabancı-anahtar column adlarını tahmin etmeye çalışmaz. Bunları attribute içinde kendin
belirtirsin (`from` / `to`); böylece bağlantı açıkça görünür olur ve bir yazım hatası sessiz bir
uyuşmazlığa değil, bir derleme hatasına yol açar. Her relation sayfası, her argümanın tam olarak hangi
column'a karşılık geldiğini ayrıntısıyla anlatıyor; ama kısaca özetlemek gerekirse:

- **`belongs_to`** — `from`, **bu** tablodaki yabancı-anahtar column'udur (zorunlu); `to` ise hedefte
  referans verilen column'dur (varsayılan olarak hedefin `id`'si).
- **`has_many` / `has_one`** — `to`, **hedef** tablodaki yabancı-anahtar column'udur (zorunlu); `from`
  ise onun geri işaret ettiği yerel column'dur (varsayılan olarak bu tablonun `id`'si).
- **`many_to_many`** — `via`, bağlantı entity'sidir; `from` ve `to` ise bağlantının sırasıyla bu
  tabloya ve hedefe işaret eden iki yabancı-anahtar column'udur.

## Sıradaki nereye

- [has_many](has-many) — birçok çocuğu olan bir ebeveyn.
- [belongs_to](belongs-to) — `has_many`'nin çocuk tarafı.
- [has_one](has-one) — bire-bir bağlantı.
- [many_to_many](many-to-many) — bir bağlantı tablosu üzerinden birbirine bağlanan iki tablo.
