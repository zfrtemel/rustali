# has_one

Sahip olan taraftan bir **bire-bir** ilişki: bu satırın başka bir tabloda tam olarak bir ilişkili
satırı vardır ve o diğer tablo yabancı anahtarı tutar. Bir yazarın bir profili vardır; bir
kullanıcının bir ayar kümesi vardır.

`has_one`, [`has_many`](has-many)'nin tek-satır ikizidir: aynı sahip olan taraf, aynı yabancı-anahtar
yerleşimi, ama ilişkili tablonun en fazla bir eşleşen satırı vardır; bu yüzden bir `Vec` yerine bir
`Option` elde edersin.

## Bildirmek

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
    pub author_id: i64,   // yabancı anahtar burada yaşar
    pub bio: String,
}
```

Attribute, `Author`'a (sahibe) gelir. `to = author_id`, **hedef** tablodaki yabancı-anahtar
kolonunu (`profiles.author_id`) adlandırır.

### Argümanlar

| Argüman | Anlamı | Varsayılan |
|---|---|---|
| *(ilk, konumsal)* | Hedef varlık tipi (`Profile`). | — (zorunlu) |
| `to = column` | **Hedef** tablodaki yabancı-anahtar kolonu. | — (zorunlu) |
| `from = column` | Geri işaret ettiği yerel kolon. | bu tablonun birincil anahtarı (`id`) |
| `as = name` | Üretilen metot adını geçersiz kıl. | hedefin snake-case'i (`profile`) |

`has_many` ile aynı argümanlar — yalnızca kardinalite farklı.

## Üretilen metotlar

`#[has_one(Profile, to = author_id)]` için, derive `Author`'a şunları ekler:

| Metot | Döndürdüğü | Sorgular |
|---|---|---|
| `author.profile(&db)` | `Option<Profile>` | bir (tembel) |
| `Author::with_profile(authors, &db)` | `Vec<(Author, Option<Profile>)>` | grup için bir (istekli) |

## Tembel ve istekli

```rust
// Tembel — bir yazarın profili
let author = Author::find_or_404(&db, id).await?;
let profile = author.profile(&db).await?;   // Option<Profile>

// İstekli — birçok yazar, profilleri tek bir ek sorguda
let authors = Author::all(&db).await?;
let paired = Author::with_profile(authors, &db).await?;
for (author, profile) in &paired {
    if let Some(p) = profile {
        println!("{}: {}", author.name, p.bio);
    }
}
```

## has_one ile belongs_to karşılaştırması

İkisi de `Option<T>` döndürür; peki hangisini kullanırsın? Bu, **hangi tablonun yabancı anahtarı
tuttuğuna** iner:

- Anahtarı *olmayan* tablo `has_one` kullanır (burada `Author`).
- Anahtarı *olan* tablo [`belongs_to`](belongs-to) kullanır (burada, `author_id`'si olan `Profile`).

Tam olarak `has_many` / `belongs_to`'daki gibi, her iki yönde de gezinmek için ikisini de bildir.

Sıradaki: [many_to_many](many-to-many).
