# CSRF & Oturumlar

Oturumlar, ziyaretçi başına veriyi imzalanmış bir çerezde saklar. Onların üzerine Rustali, flash
mesajları, formlar için eski girdi ve CSRF koruması sağlar — sunucuda render edilen bir uygulamanın
ihtiyaç duyduğu parçalar. Bunlar `sessions` özelliğinden (varsayılan olarak açık) ve
`SessionProvider`'dan gelir.

## Oturumları etkinleştirmek

Oturumların, yapılandırma mevcutken çalışan bir katmana ihtiyacı var; bu yüzden hem provider'ı hem
de katmanları ekle:

```rust
App::builder()
    .provider(SessionProvider)
    .configure(session_layers)    // çerez oturumu + CSRF katmanları
    .controllers()
    .run()
    .await
```

Onları `.env`'de yapılandır:

```bash
# .env
SESSION_COOKIE=rustali_session
SESSION_LIFETIME_MINUTES=120     # hareketsizlik sona ermesi
SESSION_SAME_SITE=lax            # lax | strict | none
SESSION_CSRF=true                # CSRF koruması açık/kapalı
```

Çerezler `HttpOnly`'dir, yapılandırıldığı gibi `SameSite`'tır, `app.key` ile imzalanır ve üretimde
`Secure`'dür. Depo şimdilik bellek içidir (tek bir düğüm için uygun); paylaşılan bir depo gelecekteki
bir iştir.

### Çerez, alan alan

| Ayar | Değer | Neden |
|---|---|---|
| `HttpOnly` | her zaman | JavaScript çerezi okuyamaz; böylece bir XSS hatası oturumu çalamaz. |
| `SameSite` | `same_site` yapılandırması | `lax` (varsayılan) çerezi çoğu siteler arası istekte engeller; `strict` daha katıdır; `none` siteler arasına izin verir (ve `Secure`'u zorlar). |
| imzalı | `app.key` ile | istemci çerezi algılanmadan kurcalayamaz. |
| `Secure` | üretimde | çerez yalnızca geliştirme dışında HTTPS üzerinden gönderilir. |

Depo süreç başına ve bellek içi olduğu için, oturumlar bir yeniden başlatmada hayatta kalmaz ve
henüz örnekler arasında paylaşılmaz — bellek [önbelleğiyle](../deeper/cache) aynı uyarı.

## Oturumu kullanmak

Ham `Session` extractor'ı rastgele anahtarlanmış değerleri okur ve yazar. Değerler JSON üzerinden
gidip gelir; böylece herhangi bir `Serialize + DeserializeOwned` tipi çalışır — `get`, eksik bir
anahtar için `None` döndürür:

```rust
#[get("/visit")]
async fn visit(session: Session) -> Result<String> {
    let count: i64 = session.get("count").await?.unwrap_or(0);
    session.insert("count", count + 1).await?;
    Ok(format!("visit #{}", count + 1))
}
```

Aşağıdaki flash, eski-girdi ve CSRF yardımcılarının hepsi bu aynı oturumun üzerine kuruludur — aksi
hâlde `get`/`insert` ile elle yazacağın desenler için kolaylıklardır.

## Flash mesajları

`Flash`, tam olarak bir yönlendirmede hayatta kalan tek istekli mesajları kuyruğa alır —
post-redirect-get deseni.

```rust
#[post("/posts")]
async fn store(db: Db, flash: Flash, /* ... */) -> Result<Redirect> {
    // ... gönderiyi oluştur ...
    flash.success("Post published.").await;
    Ok(Redirect::to("/"))
}
```

Düzeyler: `success`, `info`, `warning`, `error` (`flash.success(..)`, `flash.error(..)` vb.). Her
biri bir `level` ve `text` taşıyan bir `Message` kuyruğa alır.

Tam post-redirect-get döngüsü: eylem bir mesaj kuyruğa alır ve yönlendirir; sonraki sayfa kuyruğa
alınmış mesajları `take()`'ler (bu onları da temizler, böylece bir yenileme onları tekrar
göstermez) ve şablon onları render eder.

```rust
// 1. eylem bir mesaj kuyruğa alır, sonra yönlendirir
#[post("/posts")]
async fn store(db: Db, flash: Flash, /* ... */) -> Result<Redirect> {
    // ... gönderiyi oluştur ...
    flash.success("Post published.").await;
    Ok(Redirect::to("/"))
}

// 2. indiği sayfa kuyruğu görünümüne boşaltır
#[get("/")]
async fn index(flash: Flash) -> Result<View<Home>> {
    Ok(View::new(Home {
        flash: flash.take().await,        // Vec<Message>, bu çağrıdan sonra temizlenir
        // ...
    }))
}
```

```rust
// görünüm struct'ında
struct Home {
    flash: Vec<rustali::session::Message>,
    // ...
}
```

```html
{% for message in flash %}
  <div class="flash {{ message.level }}">{{ message.text }}</div>
{% endfor %}
```

<code v-pre>{{ message.level }}</code>, `success` / `info` / `warning` / `error` olarak render
edilir (`Level` enum'u `Display`'i uygular); böylece bir CSS sınıfı olarak da iş görür.

## Eski girdi ve hatalar

Bir [doğrulanmış](../basics/validation) form başarısız olduğunda, extractor hataları ve gönderilen
değerleri flash'lar ve geri yönlendirir. Formu tekrar render ederken onları oku:

```rust
let errors = flash.errors().await;             // ValidationErrors
let old_title = flash.old("title").await;      // Option<String>
errors.first("title");                          // Option<&str>
flash.clear_old().await;                        // bir kez render ettikten sonra
```

## CSRF koruması

`csrf = true` ile, her **güvensiz** istek (`POST`, `PUT`, `PATCH`, `DELETE`) geçerli bir token
taşımalıdır. Güvenli metotlar (`GET`, `HEAD`, `OPTIONS`, `TRACE`) ve `/api/*` yolları — bunlar
çerezlerle değil, bearer token'larıyla kimlik doğrular — muaftır. Eksik ya da yanlış bir token
`403` ile reddedilir.

Token'ı `Csrf` ile bir forma göm:

```rust
#[get("/posts/new")]
async fn new_form(csrf: Csrf) -> Result<View<NewPost>> {
    Ok(View::new(NewPost { csrf_field: csrf.field().await }))
}
```

```html
<form method="post" action="/posts">
  {{ csrf_field|safe }}
  <!-- ... -->
</form>
```

`csrf.field()`, gizli bir `<input name="_token" ...>` render eder. JavaScript istekleri için, bunun
yerine ham `csrf.token().await` değerini `X-CSRF-Token` başlığında gönder.

### Kontrol nasıl çalışır

Token, ilk ihtiyaç duyulduğunda ziyaretçinin oturumunda saklanan rastgele bir değerdir. Güvensiz bir
istekte katman, gönderilen token'ı (`_token` alanından ya da `X-CSRF-Token` başlığından)
oturumdakiyle karşılaştırır; bir eşleşme geçer, başka her şey bir 403'tür. Token oturumda yaşadığı
için — imzalanmış çereze bağlı — başka bir sitedeki bir saldırgan bir isteği sahte olarak oluşturmak
için onu okuyamaz, ki CSRF korumasının tüm amacı budur.

### Bir tek sayfalı uygulamadan ya da fetch()'ten

Sayfa sunucuda render edilen bir `<form>` yerine JavaScript tarafından yönlendirildiğinde, token'ı
bir kez oku ve onu her değiştiren istekte bir başlık olarak gönder:

```rust
// token'ı sayfaya göster (örn. bir <meta> etiketinde ya da bir JSON bootstrap'ta)
#[get("/app")]
async fn app_shell(csrf: Csrf) -> Result<View<AppShell>> {
    Ok(View::new(AppShell { csrf_token: csrf.token().await }))
}
```

```js
// sonra istemcide
fetch("/posts", {
  method: "POST",
  headers: { "Content-Type": "application/json", "X-CSRF-Token": token },
  body: JSON.stringify({ title, body }),
});
```

Bearer token'larıyla kimlik doğrulayan saf bir API bunların hiçbirine ihtiyaç duymaz: `/api/*`
yolları muaftır, çünkü bir bearer token tarayıcı tarafından bir çerez gibi otomatik gönderilmez;
yani sahte olarak oluşturulacak bir şey yoktur.

Form DTO'n tüm gövdeyi ayrıştırdığı için, token alanını kabul et; böylece ayrıştırmayı bozmaz:

```rust
#[derive(Deserialize, Validate)]
struct CreatePost {
    #[rule(required, length(min = 3, max = 200))]
    title: String,
    #[serde(default, rename = "_token")]
    _token: String,
}
```

> blog ve auth-app örnekleri bunların hepsini birlikte kullanır — flash, eski girdi, CSRF — ve bir
> entegrasyon testi, eksik bir token için 403 dahil tüm akışı sürer.
