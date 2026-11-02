# Request'ler & Validation

Bir handler, request gövdesini bir extractor üzerinden alır. Güvenmek istediğin girdiler için
**validation'dan geçen** extractor'ları kullan: kurallarını DTO'nda `#[derive(Validate)]` ile
tanımla, ardından `Validated<Json<T>>` ya da `Validated<Form<T>>` iste. Geçersiz girdi hiçbir
zaman handler'ına ulaşmaz.

## Kuralları bildirmek

```rust
use rustali::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Validate)]
struct CreatePost {
    #[rule(required, length(min = 3, max = 200))]
    title: String,

    #[rule(required, length(min = 1))]
    body: String,

    #[rule(email)]
    author_email: Option<String>,

    #[rule(range(min = 1, max = 5))]
    rating: u8,

    #[rule(one_of("draft", "published"))]
    status: String,

    #[rule(required, length(min = 8), confirmed)]
    password: String,
    #[serde(default)]
    password_confirmation: String,
}
```

### Kurallar

| Kural | Kontrol ettiği |
|---|---|
| `required` | Boş ya da yalnızca boşluk değil (dizeler), `Some` (option'lar). |
| `length(min = N, max = N)` | Karakter sayısı belirtilen aralıkta (iki sınır da isteğe bağlı). |
| `range(min = N, max = N)` | Sayısal değer belirtilen aralıkta. |
| `email` | Geçerli bir e-posta biçimi. |
| `url` | Bir `http`/`https` URL'si. |
| `alpha_num` | Yalnızca harf ve rakam. |
| `one_of("a", "b")` | Değer, listelenen seçeneklerden biri. |
| `confirmed` | `<field>_confirmation` alanıyla aynı. |
| `custom = "path"` | `fn(&T) -> Result<(), impl Display>` imzalı bir fonksiyon. |
| `nested` | Alanın kendisi `Validate`'i uygular. |

Birden fazla kuralı tek bir attribute'ta toplayabilirsin: `#[rule(required, length(min = 3, max = 200))]`.
Bir `Option<T>` alanında, `required` dışındaki her kural **değer `None` ise atlanır** — yani
`#[rule(email)] author_email: Option<String>`, e-postayı yalnızca bir değer verildiğinde doğrular.
Tüm alanlardaki bütün hatalar toplanır, yalnızca ilki değil.

### Özel kurallar

```rust
#[derive(Deserialize, Validate)]
struct Comment {
    #[rule(custom = "no_links")]
    body: String,
}

fn no_links(body: &String) -> Result<(), String> {
    if body.contains("http") { Err("Links are not allowed.".into()) } else { Ok(()) }
}
```

### İç içe validation

`#[rule(nested)]`, kendisi de bir `Validate` tipi olan bir alanı doğrular; böylece her şeyi tek bir
struct'a düzleştirmek yerine DTO'ları iç içe kullanabilirsin:

```rust
#[derive(Deserialize, Validate)]
struct Address {
    #[rule(required)]
    city: String,
    #[rule(required, length(min = 4, max = 10))]
    postcode: String,
}

#[derive(Deserialize, Validate)]
struct Order {
    #[rule(required)]
    item: String,
    #[rule(nested)]
    address: Address,
}
```

İç içe struct'tan gelen hatalar üst struct'ın altında anahtarlanır; böylece hatalı bir posta kodu
`address.postcode` olarak geri döner ve istemci her mesajı doğrudan ilgili form alanına
eşleyebilir.

## Bir handler'da kullanmak

### JSON API'leri

```rust
#[post("/api/posts")]
async fn store(db: Db, Validated(Json(input)): Validated<Json<CreatePost>>) -> Result<Json<Post>> {
    let post = Post::new(&input.title, &input.body).insert(&db).await?;
    Ok(Json(post))
}
```

Validation başarısız olduğunda istemci, alan hatalarını `errors` altında taşıyan bir RFC 9457
problem belgesiyle birlikte `422 Unprocessable Entity` alır:

```json
{
  "title": "Unprocessable Entity",
  "status": 422,
  "detail": "The given data was invalid.",
  "errors": {
    "title": ["The title must be between 3 and 200 characters."],
    "rating": ["The rating must be between 1 and 5."]
  }
}
```

### HTML formları

`Validated<Json<T>>` ve `Validated<Form<T>>` birbirinden ayrı iki tiptir; bu yüzden davranışı,
handler'ında hangisini istediğine göre belirlersin. Form varyantı başarısızlıkta farklı davranır:
hataları ve gönderilen değerleri session'a flash'lar ve request'i **geri yönlendirir** — varsa
`Referer`'a, yoksa `/`'a — böylece form, hata mesajlarıyla birlikte yeniden render edilir. (Session
paketi kurulu değilse 422 sayfasına döner. Bir `/api/*` yoluna yapılan ya da JSON isteyen bir
request ise form varyantı üzerinden bile her zaman 422 belgesini alır.)

```rust
#[post("/posts")]
async fn store(
    db: Db,
    flash: Flash,
    Validated(Form(input)): Validated<Form<CreatePost>>,
) -> Result<Redirect> {
    Post::new(input.title.trim(), input.body.trim()).insert(&db).await?;
    flash.success("Post published.").await;
    Ok(Redirect::to("/"))
}
```

Yönlendirme sonrası request'te hataları ve eski girdiyi geri okumak [View'lar](views) ile
[CSRF & Session'lar](../security/csrf) bölümlerinde ele alınır. Kısaca:

```rust
let errors = flash.errors().await;          // ValidationErrors
let old_title = flash.old("title").await;    // Option<String>
errors.first("title");                        // Option<&str>
```

## Düz (validation'sız) gövdeler

Kurallara ihtiyacın yoksa, çıplak `Json<T>` / `Form<T>` extractor'ları gövdeyi doğrudan
ayrıştırır. Gerektiğinde yine de elle bir hata döndürebilirsin:

```rust
#[post("/")]
async fn store(db: Db, Json(input): Json<CreateTodo>) -> Result<(StatusCode, Json<Todo>)> {
    if input.title.trim().is_empty() {
        return Err(Error::bad_request("title must not be empty"));
    }
    let todo = Todo::new(input.title.trim()).insert(&db).await?;
    Ok((StatusCode::CREATED, Json(todo)))
}
```

## `ValidationErrors` tipi

Hem extractor hem de elle yaptığın kontroller bir `ValidationErrors` eşlemesi (alan → mesajlar)
üretir:

| Metot | Döndürdüğü |
|---|---|
| `is_empty()` | hiçbir hata yoksa `true` |
| `has(field)` | bir alanın hatası olup olmadığı |
| `first(field)` | bir alanın ilk mesajı, `Option<&str>` |
| `get(field)` | bir alanın tüm mesajları, `&[String]` |
| `all()` | tüm mesajlar, düz bir liste olarak |

Mesajlar şu an düz İngilizcedir ("The title field is required.") ve ilerleyen zamanda i18n paketi
üzerinden çevrilebilir hâle gelecek.
