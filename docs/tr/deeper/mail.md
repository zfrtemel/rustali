# E-posta

Bir mesajı `Mailable`'ı uygulayarak tanımlar, ardından kayıtlı mailer üzerinden gönderirsin. Üç
taşıyıcı birlikte geliştirmeyi, testleri ve üretimi kapsar.

## Kurulum

`MailProvider` ekle; onu `.env`'de yapılandır:

```rust
App::builder().provider(MailProvider).controllers().run().await
```

```bash
# .env
MAIL_TRANSPORT=log                 # log | memory | smtp
MAIL_FROM=no-reply@example.com
MAIL_URL=smtp://localhost:1025     # MAIL_TRANSPORT=smtp olduğunda kullanılır
```

- **log** (varsayılan) — her mesajı göndermek yerine `tracing`'e yazar. SMTP sunucusu olmadan neyin
  gönderileceğini görürsün.
- **memory** — mesajları, test iddiaları için bellekte tutar.
- **smtp** — SMTP üzerinden gerçek teslimat (`smtp` özelliğini gerektirir, varsayılan olarak açıktır).

## Bir mesaj tanımlamak

```rust
use rustali::prelude::*;

struct Welcome {
    name: String,
}

impl Mailable for Welcome {
    fn subject(&self) -> String { "Welcome!".into() }
    fn html(&self) -> String { format!("<h1>Hi {}</h1>", self.name) }
    // fn text(&self) -> Option<String> { ... }       // isteğe bağlı düz metin alternatifi
    // fn reply_to(&self) -> Option<String> { ... }   // isteğe bağlı Reply-To
}
```

### Mailable trait'i

| Metot | Zorunlu | Döndürdüğü |
|---|---|---|
| `subject(&self)` | evet | `String` — konu satırı. |
| `html(&self)` | evet | `String` — HTML gövdesi. |
| `text(&self)` | hayır (varsayılan `None`) | `Option<String>` — düz metin alternatifi. |
| `reply_to(&self)` | hayır (varsayılan `None`) | `Option<String>` — bir `Reply-To` adresi. |

HTML'in yanında bir düz metin sürümü de istiyorsan `text` sağla (teslim edilebilirlik ve yalnızca
metin gösteren istemciler için iyidir); sağlamazsan mesaj yalnızca HTML olur.

### Gövdeyi bir template ile render etmek

Gövde sadece bir `String`'tir; dolayısıyla onu istediğin gibi render edebilirsin — bir
[Askama template'i](../basics/views) ile bile. Her `askama::Template`'in `String` döndüren bir
`render()` metodu vardır; böylece bir mailable bir template kullanıp onu `html` içinde render edebilir:

```rust
use askama::Template;

#[derive(Template)]
#[template(path = "mail/welcome.html")]
struct WelcomeHtml<'a> { name: &'a str }

struct Welcome { name: String }

impl Mailable for Welcome {
    fn subject(&self) -> String { "Welcome!".into() }
    fn html(&self) -> String {
        WelcomeHtml { name: &self.name }.render().unwrap_or_default()
    }
}
```

Böylece e-posta işaretlemesi Rust dizelerinin içine gömülü kalmaz, sayfa view'larının yanında
`templates/` altındaki bir template dosyasında durur.

## Gönderme

Mailer'a `ctx.mailer()` (ya da `Inject<SharedMailer>`) ile eriş ve bir alıcıya gönder; `from`
belirtmezsen yapılandırılmış adres kullanılır:

```rust
#[post("/signup")]
async fn signup(ctx: Ctx, db: Db, /* ... */) -> Result<Redirect> {
    let user = /* kullanıcı oluştur */;
    ctx.mailer()?.send(&user.email, &Welcome { name: user.name.clone() }).await?;
    Ok(Redirect::to("/dashboard"))
}
```

Bir mesajda göndereni değiştirmek için `send_from(from, to, &mail)` kullan.

## Arka planda gönderme

E-posta göndermek yavaştır; onu bir [job](queues) içinden göndererek request yolunun dışında tut:

```rust
#[derive(Serialize, Deserialize)]
struct SendWelcome { user_id: i64 }

#[async_trait]
impl Job for SendWelcome {
    const NAME: &'static str = "send_welcome";
    async fn handle(&self, ctx: &AppContext) -> Result<()> {
        let user = User::find_or_404(&*ctx.db()?, self.user_id).await?;
        ctx.mailer()?.send(&user.email, &Welcome { name: user.name }).await
    }
}

// handler'da:
ctx.queue()?.dispatch(SendWelcome { user_id: user.id }).await?;
```

## Ortam başına bir taşıyıcı seçmek

Taşıyıcı bir yapılandırma değeridir; dolayısıyla her ortam kendi taşıyıcısını kod değiştirmeden seçer:

- **geliştirme** — `log`, böylece ayakta bir SMTP sunucusu olmadan render edilen mesajı terminalinde
  görürsün.
- **testler** — `memory`, böylece tam olarak neyin gönderildiğini doğrulayabilirsin (aşağıya bak).
- **üretim** — `smtp`, `url` üzerinden gerçek e-posta servisine yönlendirilmiş.

Mailer tek bir `send` çağrısının arkasında durduğu için taşıyıcıyı değiştirmek, mesajları kuran ve
gönderen koda hiç dokunmaz.

## E-postayı test etmek

`MAIL_TRANSPORT=memory` ayarla (ya da bir `MemoryTransport` kur) ve neyin gönderildiğini doğrula.
Bir `MemoryTransport`, teslim edilen her mesajı kaydeder:

```rust
let transport = MemoryTransport::new();
// ... gönderen kodu çalıştır ...
assert_eq!(transport.count(), 1);
let sent = transport.sent();                 // Vec<Email>
assert_eq!(sent[0].to, "ada@example.com");
assert!(sent[0].subject.contains("Welcome"));
```

`sent()`, teslim edilen her `Email`'i döndürür (`from`, `to`, `subject`, `html` ve isteğe bağlı
`text` alanlarıyla); `count()` ise teslim edilen mesaj sayısını verir.

## Kapsam

Mailer en yaygın senaryoyu kapsar: tek bir alıcı, isteğe bağlı düz metin sürümü olan bir HTML gövdesi
ve isteğe bağlı bir `Reply-To`. CC/BCC'yi, birden fazla alıcıyı ya da ekleri modellemez — bunlara
ihtiyacın olursa doğrudan `lettre`'yi kullan (SMTP taşıyıcısı da onun üzerine kuruludur).
