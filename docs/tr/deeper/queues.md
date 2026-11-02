# Queue'lar & Job'lar

Yavaş işleri (e-posta, görüntü işleme, üçüncü taraf bir API'yi çağırma) request yolundan çıkar. İşi
bir `Job` olarak tanımla, bir handler'dan gönder, arka planda bir worker çalıştırsın.

## Bir job tanımlamak

Bir job, `Job`'ı uygulayan ve serileştirilebilir bir struct'tır. Kendine bir ad verir, isteğe bağlı
olarak bir yeniden deneme politikası belirler ve asıl işini `handle` içinde yapar — `handle` da tam
`AppContext`'i alır.

```rust
use rustali::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SendWelcome {
    user_id: i64,
}

#[async_trait]
impl Job for SendWelcome {
    const NAME: &'static str = "send_welcome";          // benzersiz, job'u geri eşlemeye yarar
    const RETRY: Retry = Retry::times(3, Duration::from_secs(5));   // isteğe bağlı

    async fn handle(&self, ctx: &AppContext) -> Result<()> {
        let user = User::find_or_404(&*ctx.db()?, self.user_id).await?;
        ctx.mailer()?.send(&user.email, &Welcome { name: user.name }).await
    }
}
```

### Yeniden denemeler

`RETRY`, `handle` bir `Err` döndürdüğünde ne olacağını belirler:

| Değer | Denemeler | Son başarısızlıkta |
|---|---|---|
| `Retry::NONE` (varsayılan) | 1 | loglanır ve atılır |
| `Retry::times(n, backoff)` | 1 + `n` | loglanır ve atılır |

`times(n, ...)`, ilk denemenin **üzerine** `n` yeniden deneme demektir; yani `Retry::times(3, ...)`
toplamda dört çalıştırmaya kadar çıkar. Backoff, denemeler arasındaki **sabit** gecikmedir (üstel
backoff ileride eklenebilir). Yeniden deneme hakkını tüketen bir job loglanır ve atılır — henüz bir
ölü-mektup (dead-letter) deposu yok — bu yüzden yeniden denemeleri bir garanti değil, en iyi çabayla
sağlanan bir tampon olarak düşün.

### Job'ları yeniden denemesi güvenli olacak şekilde yaz

Bir job birden fazla kez çalışabileceği için (bir yeniden deneme yüzünden ya da — ileride dayanıklı
bir backend geldiğinde — en az bir kez teslimat nedeniyle), önemli olduğu yerlerde `handle`'ı
**idempotent** yap: aynı e-postayı iki kez göndermeye, iki kez ücretlendirmeye ya da yinelenen bir
satır eklemeye karşı koru. Tekrar çalışması güvenli bir job, yeniden denendiğinde kafa yormana gerek
bırakmayan bir job'tur.

## Job'ları ve worker'ı kaydetmek

Bir job gönderildiğinde **serileştirilir** (çünkü `Serialize`'dır), `NAME`'iyle etiketlenir ve
queue'ya konur. Worker, doğru tipi seçip onu geri **deserialize** etmek için bu adı okur — bu yüzden
`NAME` tüm job'lar arasında benzersiz olmalı ve bir job'un alanlarını değiştirmek, zaten queue'ya
alınmış her şey açısından bir format değişikliğidir. Worker'ın her adı doğru job'a yönlendirmesi için,
tüm job tiplerini listeleyen bir `Worker` kur ve onu `QueueProvider`'a geçir:

```rust
let worker = Worker::builder()
    .register::<SendWelcome>()
    .register::<GenerateThumbnail>()
    .build();

App::builder()
    .provider(DatabaseProvider)
    .provider(MailProvider)
    .provider(QueueProvider::new(worker))
    .controllers()
    .run()
    .await
```

## Gönderme

Bir handler içinde queue'ya `ctx.queue()` (ya da `Inject<SharedQueue>`) ile eriş ve job'u gönder:

```rust
#[post("/signup")]
async fn signup(ctx: Ctx, db: Db, /* ... */) -> Result<Redirect> {
    let user = /* kullanıcı oluştur */;
    ctx.queue()?.dispatch(SendWelcome { user_id: user.id }).await?;
    Ok(Redirect::to("/dashboard"))
}
```

Worker'ın tanımadığı bir job tipi deserialize edilemez: worker, yanlış kodu çalıştırmak yerine net
bir "`<name>` job'u için kayıtlı handler yok" hatası verir. Yani yeni bir job tipi eklemek, bir
`.register::<T>()` satırı eklemekten ibarettir.

## Worker'ı çalıştırmak

Queue'ya alınmış job'ları, `QueueProvider`'ın kaydettiği konsol komutuyla işle:

```bash
cargo run -- queue:work        # Ctrl-C'ye kadar queue'yu boşaltır
```

Geliştirmede worker'ı `cargo run`'ın yanında çalıştırabilirsin; üretimde ise ayrı bir süreç olarak
çalıştır. Worker ile web süreci aynı `App::builder()`'ı paylaşır; böylece bir job'un `handle`'ı, bir
request handler'ıyla tam olarak aynı bağlama sahip olur — veritabanı, mailer, servislerin.

## Bugün elindekiler

Yerleşik queue **süreç içidir**: gönderilen job'lar, `queue:work`'ün boşalttığı bir kanaldan geçer.
Job'lar request'in ötesine geçer ama **bir süreç yeniden başlatmasının ötesine geçmez**. Dayanıklı
bir veritabanı/Redis backend'i ileride aynı `Queue` trait'inin arkasına takılacak — job kodun
değişmez. Şimdilik, bir yeniden başlatmada kaybını göze alamayacağın job'lar için bunu kullanma.

> `rustali make:job <Name>`, bir job oluşturur.
