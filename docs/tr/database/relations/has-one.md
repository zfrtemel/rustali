# has_one

Sahip olan taraftan tanımlanan bir **bire-bir** relation: bu row'un başka bir tabloda tam olarak bir
ilişkili row'u vardır ve yabancı anahtarı o diğer tablo tutar. Bir yazarın bir profili olur; bir
kullanıcının bir ayar kümesi olur.

`has_one`, [`has_many`](has-many)'nin tek-row'luk ikizidir: sahip olan taraf aynı, yabancı anahtarın
yerleşimi aynı; tek fark, ilişkili tabloda en fazla bir eşleşen row bulunmasıdır. Bu yüzden bir `Vec`
yerine bir `Option` elde edersin.

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

Attribute, sahip olan tarafa, yani `Author`'a yazılır. `to = author_id`, **hedef** tablodaki
yabancı-anahtar column'unu (`profiles.author_id`) belirtir.

### Argümanlar

| Argüman | Anlamı | Varsayılan |
|---|---|---|
| *(ilk, konumsal)* | Hedef entity tipi (`Profile`). | — (zorunlu) |
| `to = column` | **Hedef** tablodaki yabancı-anahtar column'u. | — (zorunlu) |
| `from = column` | Bunun geri işaret ettiği yerel column. | bu tablonun primary key'i (`id`) |
| `as = name` | Üretilen metot adını değiştirir. | hedefin snake-case'i (`profile`) |

Argümanlar `has_many` ile birebir aynıdır — tek fark kardinalitedir.

## Üretilen metotlar

`#[has_one(Profile, to = author_id)]` için, derive `Author`'a şunları ekler:

| Metot | Döndürdüğü | Query'ler |
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

İkisi de `Option<T>` döndürür; peki hangisini kullanmalı? Bu, tamamen **yabancı anahtarı hangi
tablonun tuttuğuna** bağlıdır:

- Anahtarı *tutmayan* tablo `has_one` kullanır (burada `Author`).
- Anahtarı *tutan* tablo [`belongs_to`](belongs-to) kullanır (burada, `author_id`'yi taşıyan `Profile`).

Tıpkı `has_many` / `belongs_to`'da olduğu gibi, her iki yöne de gidebilmek için ikisini birden
bildir.

Sıradaki: [many_to_many](many-to-many).
