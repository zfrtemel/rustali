# E-posta

`Mailable`'ı uygulayarak bir mesaj tanımla, sonra onu kayıtlı mailer aracılığıyla gönder. Üç
taşıyıcı geliştirmeyi, testleri ve üretimi kapsar.

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

- **log** (varsayılan) — her mesajı göndermek yerine `tracing`'e yazar. Bir SMTP sunucusu olmadan
  neyin gideceğini gör.
- **memory** — test iddiaları için mesajları bellekte kaydeder.
- **smtp** — SMTP aracılığıyla gerçek teslimat (`smtp` özelliğine ihtiyaç duyar, varsayılan olarak
  açık).

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
| `text(&self)` | hayır (varsayılan `None`) | `Option<String>` — bir düz metin alternatifi. |
| `reply_to(&self)` | hayır (varsayılan `None`) | `Option<String>` — bir `Reply-To` adresi. |

HTML'in yanında bir düz metin parçası istediğinde `text` sağla (teslim edilebilirlik ve yalnızca
metin istemcileri için iyi); onu dışarıda bırak, mesaj yalnızca HTML olur.

### Gövdeyi bir şablonla render etmek

Gövde yalnızca bir `String`'tir; bu yüzden onu istediğin gibi render et — bir
[Askama şablonu](../basics/views) dahil. Herhangi bir `askama::Template`'in bir `String` döndüren bir
`render()`'ı vardır; böylece bir mailable bir şablona sahip olabilir ve onu `html`'de render
edebilir:

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

Bu, e-posta işaretlemesini Rust dizelerinde satır içinde değil, sayfa görünümlerinin yanında
`templates/` altında bir şablon dosyasında tutar.

## Gönderme

Mailer'a `ctx.mailer()` (ya da `Inject<SharedMailer>`) ile eriş ve bir alıcıya gönder; `from`,
yapılandırılmış adrese varsayılan olur:

```rust
#[post("/signup")]
async fn signup(ctx: Ctx, db: Db, /* ... */) -> Result<Redirect> {
    let user = /* kullanıcı oluştur */;
    ctx.mailer()?.send(&user.email, &Welcome { name: user.name.clone() }).await?;
    Ok(Redirect::to("/dashboard"))
}
```

Bir mesaj için göndereni geçersiz kılmak üzere `send_from(from, to, &mail)` kullan.

## Arka planda gönderme

E-posta yavaştır; onu bir [iş](queues)'ten göndererek istek yolundan uzak tut:

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

Taşıyıcı bir yapılandırma değeridir; bu yüzden her ortam bir kod değişikliği olmadan kendininkini
seçer:

- **geliştirme** — `log`, böylece çalıştıracak bir SMTP sunucusu olmadan render edilen mesajı
  terminalinde görürsün.
- **testler** — `memory`, böylece tam olarak neyin gönderildiğini iddia edebilirsin (aşağıya bak).
- **üretim** — `smtp`, `url` aracılığıyla gerçek e-posta servisine yönlendirilmiş.

Mailer tek bir `send` çağrısının arkasında oturduğu için, taşıyıcıları değiştirmek mesajları kuran
ve gönderen koda asla dokunmaz.

## E-postayı test etmek

`MAIL_TRANSPORT=memory` ayarla (ya da bir `MemoryTransport` kur) ve neyin gönderildiğini iddia et.
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
`text` ile); `count()`, teslim edilen sayıdır.

## Kapsam

Mailer yaygın yolu kapsar: tek bir alıcı, isteğe bağlı bir düz metin parçası olan bir HTML gövdesi
ve isteğe bağlı bir `Reply-To`. CC/BCC'yi, birden fazla alıcıyı ya da ekleri modellemez — onlara
ihtiyacın varsa, doğrudan `lettre`'ye (SMTP taşıyıcısının üzerine kurulduğu) başvur.
