# CSRF & Session'lar

Session'lar, her ziyaretçiye ait veriyi imzalanmış bir çerezde saklar. Rustali bunun üzerine flash
mesajlarını, formlar için eski girdiyi ve CSRF korumasını ekler — sunucuda render edilen bir
uygulamanın ihtiyaç duyduğu parçalar. Bunlar `sessions` özelliğinden (varsayılan olarak açık) ve
`SessionProvider`'dan gelir.

## Session'ları etkinleştirmek

Session'lar, yapılandırmanın hazır olduğu sırada çalışan bir katmana ihtiyaç duyar; bu yüzden hem
provider'ı hem de katmanları ekle:

```rust
App::builder()
    .provider(SessionProvider)
    .configure(session_layers)    // çerez oturumu + CSRF katmanları
    .controllers()
    .run()
    .await
```

Bunları `.env`'de yapılandır:

```bash
# .env
SESSION_COOKIE=rustali_session
SESSION_LIFETIME_MINUTES=120     # hareketsizlik sona ermesi
SESSION_SAME_SITE=lax            # lax | strict | none
SESSION_CSRF=true                # CSRF koruması açık/kapalı
```

Çerezler `HttpOnly`'dir, yapılandırdığın `SameSite` değerini taşır, `app.key` ile imzalanır ve
üretimde `Secure` olur. Depo şimdilik bellek içidir (tek bir düğüm için uygundur); paylaşılan bir
depo ileride ele alınacak bir konu.

### Çerez ayarları tek tek

| Ayar | Değer | Neden |
|---|---|---|
| `HttpOnly` | her zaman | JavaScript çerezi okuyamaz; böylece bir XSS açığı session'ı çalamaz. |
| `SameSite` | `same_site` ayarı | `lax` (varsayılan) çerezi siteler arası request'lerin çoğunda engeller; `strict` daha katıdır; `none` siteler arası gönderime izin verir (ve `Secure`'u zorunlu kılar). |
| imzalı | `app.key` ile | istemci çerezi, fark edilmeden kurcalayamaz. |
| `Secure` | üretimde | çerez, geliştirme dışında yalnızca HTTPS üzerinden gönderilir. |

Depo, her sürece özel ve bellek içi olduğu için session'lar yeniden başlatmayı atlatamaz ve henüz
örnekler arasında paylaşılmaz — bellek [cache'inde](../deeper/cache) olduğu gibi aynı uyarı
geçerli.

## Session'ı kullanmak

Ham `Session` extractor'ı, istediğin anahtarla değer okur ve yazar. Değerler JSON'a çevrilip geri
alınır; böylece `Serialize + DeserializeOwned` olan her tip çalışır — `get`, olmayan bir anahtar
için `None` döndürür:

```rust
#[get("/visit")]
async fn visit(session: Session) -> Result<String> {
    let count: i64 = session.get("count").await?.unwrap_or(0);
    session.insert("count", count + 1).await?;
    Ok(format!("visit #{}", count + 1))
}
```

Aşağıdaki flash, eski girdi ve CSRF yardımcılarının hepsi bu aynı session'ın üzerine kuruludur —
yani aslında `get`/`insert` ile elle yazacağın kalıpları senin için kolaylaştıran araçlardır.

## Flash mesajları

`Flash`, yalnızca bir yönlendirme boyunca yaşayan tek request'lik mesajları queue'ya alır —
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
biri, bir `level` ve bir `text` taşıyan bir `Message`'ı queue'ya ekler.

Tam post-redirect-get döngüsü şöyle işler: eylem bir mesajı queue'ya alıp yönlendirir; sonraki sayfa
queue'daki mesajları `take()` ile alır (bu işlem onları aynı zamanda temizler, böylece sayfa
yenilendiğinde tekrar görünmezler) ve template onları render eder.

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
edilir (`Level` enum'u `Display`'i uygular); böylece doğrudan bir CSS sınıfı olarak da kullanılabilir.

## Eski girdi ve hatalar

Bir [validate edilmiş](../basics/validation) form başarısız olduğunda, extractor hataları ve
gönderilen değerleri flash'a yazıp geri yönlendirir. Formu yeniden render ederken bunları oku:

```rust
let errors = flash.errors().await;             // ValidationErrors
let old_title = flash.old("title").await;      // Option<String>
errors.first("title");                          // Option<&str>
flash.clear_old().await;                        // bir kez render ettikten sonra
```

## CSRF koruması

`csrf = true` iken her **güvensiz** request (`POST`, `PUT`, `PATCH`, `DELETE`) geçerli bir token
taşımak zorundadır. Güvenli metotlar (`GET`, `HEAD`, `OPTIONS`, `TRACE`) ile `/api/*` yolları bundan
muaftır — çünkü bu yollar çerezlerle değil, bearer token'larıyla kimlik doğrular. Eksik ya da yanlış
bir token `403` ile reddedilir.

Token'ı `Csrf` ile forma göm:

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

`csrf.field()`, gizli bir `<input name="_token" ...>` render eder. JavaScript request'lerinde ise
bunun yerine ham `csrf.token().await` değerini `X-CSRF-Token` başlığıyla gönder.

### Kontrol nasıl çalışır

Token, ilk ihtiyaç duyulduğu anda ziyaretçinin session'ında saklanan rastgele bir değerdir. Güvensiz
bir request geldiğinde katman, gönderilen token'ı (`_token` alanından ya da `X-CSRF-Token`
başlığından) session'daki değerle karşılaştırır; eşleşirse geçer, aksi her durumda 403 döner. Token
session'da — yani imzalanmış çereze bağlı olarak — durduğu için, başka bir sitedeki saldırgan sahte
bir request üretmek için onu okuyamaz; CSRF korumasının asıl amacı da budur.

### Bir tek sayfalı uygulamadan ya da fetch()'ten

Sayfa, sunucuda render edilen bir `<form>` yerine JavaScript tarafından sürüldüğünde, token'ı bir
kez oku ve değişiklik yapan her request'te onu bir başlık olarak gönder:

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

Bearer token'larıyla kimlik doğrulayan salt API'lerin bunların hiçbirine ihtiyacı yoktur: `/api/*`
yolları muaftır, çünkü bir bearer token'ı tarayıcı bir çerez gibi otomatik göndermez; dolayısıyla
sahtesi üretilebilecek bir şey de yoktur.

Form DTO'n gövdenin tamamını ayrıştırdığı için, token alanını da kabul etmelisin; yoksa bu alan
ayrıştırmayı bozar:

```rust
#[derive(Deserialize, Validate)]
struct CreatePost {
    #[rule(required, length(min = 3, max = 200))]
    title: String,
    #[serde(default, rename = "_token")]
    _token: String,
}
```

> blog ve auth-app örnekleri bunların hepsini bir arada kullanır — flash, eski girdi, CSRF — ve bir
> entegrasyon testi, eksik token için dönen 403 dahil akışın tamamını çalıştırır.
