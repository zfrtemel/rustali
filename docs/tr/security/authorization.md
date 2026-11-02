# Yetkilendirme

Yetkilendirme, "bu kullanıcının bunu yapmasına izin var mı?" sorusunu yanıtlar. Rustali bunu, bir
**Gate** üzerinden denetlenen, adlandırılmış **yetenekler** olarak modeller; bir route üzerinde
`#[authorize("...")]` ile bildirimsel biçimde uygulanır.

## Yerleşik yetenekler

Şu ikisi, kimliği doğrulanmış kullanıcı üzerinde kutudan çıktığı gibi çalışır
([Kimlik Doğrulama](authentication)'ya bak):

- `authenticated` — giriş yapmış herhangi bir kullanıcı için geçer.
- `role:<name>` — kullanıcının `roles()` listesi `<name>` içerdiğinde geçer.

```rust
#[post("/posts")]
#[authorize(role = "editor")]        // "role:editor" yeteneği için şeker
async fn store(/* ... */) -> Result<Redirect> { /* ... */ }

#[delete("/posts/{id}")]
#[authorize("authenticated")]
async fn destroy(/* ... */) -> Result<StatusCode> { /* ... */ }
```

İki yazım, tek mekanizma:

- `#[authorize("ability")]`, bir yeteneği doğrudan adıyla belirtir — `authenticated`, `role:editor`
  ya da `post.update` gibi herhangi bir özel politikanın yeteneği.
- `#[authorize(role = "editor")]`, `role:editor` yeteneğine açılan bir kısayoldur.

Filtre, request anında mevcut kullanıcıyı çözer, yeteneği değerlendirir ve kimse giriş yapmamışsa
**401**, kullanıcı var ama reddedilmişse **403** response'u döndürür — hem de handler hiç
çalışmadan. Bu ayrım istemciler açısından önemlidir: 401 "giriş yap" demek, 403 ise "giriş yaptın
ama iznin yok" demektir.

## Özel politikalar

Rollerin ötesine geçen her şey için `Policy`'yi uygula: ona bir yetenek adı ve async bir karar
fonksiyonu ver. Bu fonksiyon, bağlamı, (isteğe bağlı) aktörü ve request parçalarını içeren bir
`Check` alır.

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

| Alan | Nedir |
|---|---|
| `check.ctx` | `AppContext` (veritabanı, servisler, yapılandırma) |
| `check.actor` | `Option<&dyn Actor>` — mevcut kullanıcı (`id()`, `roles()`) |
| `check.parts` | request'in başlık bölümü (yol, başlıklar, uzantılar) |

Politikaları builder'da kaydet, sonra bir route üzerinden yeteneğe atıfta bulun:

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

Bir politika içinde somut kullanıcı tipine mi ihtiyacın var? Aktörü aşağıya cast et:

```rust
let user = check.actor.and_then(|a| a.as_any().downcast_ref::<User>());
```

### Gerçekçi bir sahiplik politikası

Çoğu özel politika, ilgili kaydı yükler ve onu aktörle karşılaştırır. Burada bir gönderiyi yalnızca
yazarı düzenleyebilir — politika id'yi yoldan okur, gönderiyi yükler ve sahipliği veritabanı
üzerinden doğrular:

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

Bir politika, reddetmek için `Ok(false)`, yalnızca gerçek bir arıza (örneğin bir veritabanı hatası)
için ise `Err(..)` döndürür. `false` ile reddetmek temiz bir 403 üretir; bir `Err` ise 500 olarak
yayılır, çünkü kullanıcının izninin olmadığını değil, bir şeylerin bozulduğunu anlatır.

## Bir handler içinde kontrol etmek

`#[authorize]`, bütün bir route'u korumanın en temiz yoludur; ama bazen karar, ancak handler'ın
ortasında elinde olan verilere bağlıdır ya da reddetmek yerine akışı dallandırmak istersin. Böyle
durumlarda kullanıcıyı bir extractor olarak al ve kararı düz Rust ile ver:

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

`Error::Forbidden` / `Error::Unauthorized` döndürmek, filtrenin vereceğiyle aynı 403 / 401'i üretir;
böylece elle yazdığın kontroller ile `#[authorize]` birbiriyle tutarlı kalır. Kural "bu kullanıcı bu
route'a hiç erişebilir mi" ise attribute'u kullan; kural yüklenen kayda bağlıysa ya da reddetmek
yerine farklı bir sonuç göstermek istiyorsan handler içinde bir kontrol kullan.

## Attribute olmadan bir politika uygulamak

`#[authorize("ability")]`, bir tower katmanına açılır; aynı katmanı bir router'a elle
`AuthorizeLayer::new("ability")` ile de ekleyebilirsin — bütün bir route grubunu tek seferde korumak
için elverişlidir:

```rust
use rustali::auth::AuthorizeLayer;

let admin = admin_routes().layer(AuthorizeLayer::new("role:admin"));
```

Bu, varsayılan middleware yığınına ihtiyaç duyar (bağlamı enjekte eden ve kullanıcıyı çözen yığın);
bu yüzden yalnızca `App::builder()` üzerinden kurulan router'larda çalışır.

## Yetkilendirme nerede oturur

`#[authorize]`, varsayılan middleware yığını içinde çalışan bir tower katmanı üretir; böylece enjekte
edilen `AppContext`'i ve çözülmüş kullanıcıyı görebilir. Bir katman olduğu için karar, handler
gövden çalışmadan önce verilir — reddedilen bir request senin koduna hiç ulaşmaz. Bilinmeyen bir
yetenek reddedilir ve loglanır; yani bir yazım hatası açık değil, kapalı tarafta hata verir
(fail-closed).

> `rustali make:policy <Name>`, senin yerine bir `Policy` iskeleti oluşturur.
