# Yetkilendirme

Yetkilendirme, "bu kullanıcının bunu yapmasına izin var mı?" sorusunu yanıtlar. Rustali bunu bir
**Gate** tarafından kontrol edilen adlandırılmış **yetenekler** olarak modeller; bir route üzerinde
`#[authorize("...")]` ile bildirimsel olarak zorlanır.

## Yerleşik yetenekler

İkisi kimliği doğrulanmış kullanıcıya karşı kutudan çıktığı gibi çalışır
([Kimlik Doğrulama](authentication)'ya bak):

- `authenticated` — herhangi bir giriş yapmış kullanıcı için geçer.
- `role:<name>` — kullanıcının `roles()`'u `<name>` içerdiğinde geçer.

```rust
#[post("/posts")]
#[authorize(role = "editor")]        // "role:editor" yeteneği için şeker
async fn store(/* ... */) -> Result<Redirect> { /* ... */ }

#[delete("/posts/{id}")]
#[authorize("authenticated")]
async fn destroy(/* ... */) -> Result<StatusCode> { /* ... */ }
```

İki söz dizimi, tek mekanizma:

- `#[authorize("ability")]`, bir yeteneği doğrudan adlandırır — `authenticated`, `role:editor` ya da
  `post.update` gibi herhangi bir özel politikanın yeteneği.
- `#[authorize(role = "editor")]`, `role:editor` yeteneğine genişleyen bir şekerdir.

İstek anında filtre, mevcut kullanıcıyı çözer, yeteneği değerlendirir ve kimse giriş yapmamışsa
**401** ya da kullanıcı mevcut ama reddedilmişse **403** yanıtını verir — handler çalışmadan önce.
Ayrım istemciler için önemlidir: 401 "giriş yap" demektir, 403 "giriş yaptın ama izin verilmedi"
demektir.

## Özel politikalar

Rollerin ötesindeki her şey için `Policy`'yi uygula: ona bir yetenek adı ve bir async karar ver.
Bağlamı, (isteğe bağlı) aktörü ve istek parçalarını içeren bir `Check` alır.

```rust
use rustali::prelude::*;

pub struct UpdatePost;

impl Policy for UpdatePost {
    fn ability(&self) -> &'static str { "post.update" }

    fn allows<'a>(&'a self, check: Check<'a>) -> AuthFuture<'a, bool> {
        Box::pin(async move {
            let Some(actor) = check.actor else { return Ok(false) };
            // örn. yalnızca id'si yolda olan sahip güncelleyebilir
            Ok(check.parts.uri.path().ends_with(&format!("/{}", actor.id())))
        })
    }
}
```

`Check` sana şunları verir:

| Alan | Olduğu |
|---|---|
| `check.ctx` | `AppContext` (veritabanı, servisler, yapılandırma) |
| `check.actor` | `Option<&dyn Actor>` — mevcut kullanıcı (`id()`, `roles()`) |
| `check.parts` | istek başlığı (yol, başlıklar, uzantılar) |

Politikaları builder'da kaydet, sonra bir route'tan yeteneğe atıfta bulun:

```rust
App::builder()
    .policy(UpdatePost)
    // ...
    .controllers()
```

```rust
#[patch("/posts/{id}")]
#[authorize("post.update")]
async fn update(/* ... */) -> Result<Json<Post>> { /* ... */ }
```

Bir politika içinde somut kullanıcı tipine mi ihtiyacın var? Aktörü aşağı dönüştür:

```rust
let user = check.actor.and_then(|a| a.as_any().downcast_ref::<User>());
```

### Gerçekçi bir sahiplik politikası

Çoğu özel politika, konuyu yükler ve onu aktörle karşılaştırır. Burada, bir gönderiyi yalnızca
yazarı düzenleyebilir — politika id'yi yoldan okur, gönderiyi yükler ve sahipliği veritabanına karşı
kontrol eder:

```rust
pub struct UpdatePost;

impl Policy for UpdatePost {
    fn ability(&self) -> &'static str { "post.update" }

    fn allows<'a>(&'a self, check: Check<'a>) -> AuthFuture<'a, bool> {
        Box::pin(async move {
            // giriş yapmış olmalı
            let Some(actor) = check.actor else { return Ok(false) };

            // eşleşen yoldan id, örn. /posts/{id}
            let Some(id) = check.parts.uri.path().rsplit('/').next().and_then(|s| s.parse::<i64>().ok())
            else {
                return Ok(false);
            };

            // konuyu yükle ve sahipleri karşılaştır
            let db = check.ctx.db()?;
            let Some(post) = Post::find(&*db, id).await? else { return Ok(false) };
            Ok(post.user_id.to_string() == actor.id())
        })
    }
}
```

Bir politika, reddetmek için `Ok(false)` ve yalnızca gerçek bir başarısızlık (örneğin bir veritabanı
hatası) için `Err(..)` döndürür. `false` döndürerek reddetmek temiz bir 403 üretir; bir `Err`, bir
500 olarak yayılır çünkü kullanıcının izni olmadığını değil, bir şeyin bozulduğunu ifade eder.

## Bir handler içinde kontrol etmek

`#[authorize]`, bütün bir route'u korumanın en temiz yoludur, ama bazen karar yalnızca handler'ın
ortasında sahip olduğun verilere bağlıdır ya da reddetmek yerine dallanmak istersin. O zaman
kullanıcıyı bir extractor olarak al ve düz Rust'ta karar ver:

```rust
#[get("/posts/{id}/edit")]
async fn edit(Auth(user): Auth<User>, db: Db, Path(id): Path<i64>) -> Result<View<EditPost>> {
    let post = Post::find_or_404(&db, id).await?;
    if post.user_id != user.id {
        return Err(Error::Forbidden);   // 403, JSON'a ya da bir hata sayfasına pazarlık edilir
    }
    Ok(View::new(EditPost { post }))
}
```

`Error::Forbidden` / `Error::Unauthorized` döndürmek, filtrenin vereceği aynı 403 / 401'i verir;
böylece elle yazılmış kontroller ve `#[authorize]` tutarlı kalır. Kural "bu kullanıcı bu route'a
hiç ulaşabilir mi" olduğunda attribute'a başvur; kural yüklenen kayda bağlı olduğunda ya da
reddetmek yerine farklı bir sonuç göstermek istediğinde handler içi bir kontrole başvur.

## Attribute olmadan bir politika uygulamak

`#[authorize("ability")]`, bir tower katmanına genişler ve o aynı katmanı bir router'a elle
`AuthorizeLayer::new("ability")` ile ekleyebilirsin — bütün bir route grubunu bir kerede korumak
için kullanışlı:

```rust
use rustali::auth::AuthorizeLayer;

let admin = admin_routes().layer(AuthorizeLayer::new("role:admin"));
```

Varsayılan middleware yığınına ihtiyaç duyar (bağlamı enjekte eden ve kullanıcıyı çözen); bu yüzden
`App::builder()` aracılığıyla kurulan router'larda çalışır.

## Yetkilendirme nerede oturur

`#[authorize]`, varsayılan middleware yığını içinde çalışan bir tower katmanı yayar; böylece enjekte
edilen `AppContext`'i ve çözülen kullanıcıyı görür. Bir katman olduğu için, karar handler
gövdenden önce gerçekleşir — reddedilen bir istek asla koduna dokunmaz. Bilinmeyen bir yetenek
reddedilir ve loglanır; böylece bir yazım hatası açık değil, kapalı başarısız olur.

> `rustali make:policy <Name>`, senin için bir `Policy` oluşturur.
