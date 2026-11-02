# Queue'lar & Job'lar

Yavaş işi (e-posta, görüntü işleme, üçüncü taraf bir API çağırma) request yolundan uzaklaştır. Onu bir
`Job` olarak tanımla, bir handler'dan gönder ve bir worker onu arka planda çalıştırsın.

## Bir job tanımlamak

Bir job, `Job`'ı uygulayan serileştirilebilir bir struct'tır. Kendini adlandırır, isteğe bağlı olarak
bir yeniden deneme politikası ayarlar ve işini `handle`'da yapar — ki o da tam `AppContext`'i alır.

```rust
use rustali::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SendWelcome {
    user_id: i64,
}

#[async_trait]
impl Job for SendWelcome {
    const NAME: &'static str = "send_welcome";          // benzersiz, işi geri yönlendirir
    const RETRY: Retry = Retry::times(3, Duration::from_secs(5));   // isteğe bağlı

    async fn handle(&self, ctx: &AppContext) -> Result<()> {
        let user = User::find_or_404(&*ctx.db()?, self.user_id).await?;
        ctx.mailer()?.send(&user.email, &Welcome { name: user.name }).await
    }
}
```

### Yeniden denemeler

`RETRY`, `handle` bir `Err` döndürdüğünde ne olacağını kontrol eder:

| Değer | Denemeler | Son başarısızlıkta |
|---|---|---|
| `Retry::NONE` (varsayılan) | 1 | loglanır ve düşürülür |
| `Retry::times(n, backoff)` | 1 + `n` | loglanır ve düşürülür |

`times(n, ...)`, ilk denemenin **ötesinde** `n` yeniden deneme anlamına gelir; yani
`Retry::times(3, ...)` toplamda dörde kadar çalıştırmadır. Backoff, denemeler arasında **sabit** bir
gecikmedir (üstel backoff daha sonra gelebilir). Yeniden denemelerini tüketen bir job loglanır ve
düşürülür — henüz bir ölü-mektup deposu yok — bu yüzden yeniden denemeleri bir garanti değil, en iyi
çaba tamponu olarak gör.

### Job'ları yeniden denemesi güvenli olacak şekilde yaz

Bir job birden fazla kez çalışabileceği için (bir retry ya da — daha sonra dayanıklı bir
backend'le — en-az-bir-kez teslimat), önemli olduğu yerde `handle`'ı **idempotent** yap: aynı
e-postayı iki kez göndermeye, iki kez ücretlendirmeye ya da yinelenen bir satır eklemeye karşı
koru. Tekrar çalıştırılması güvenli bir job, yeniden denendiğinde asla üzerinde çok düşünmek zorunda
olmadığın bir job'tur.

## Job'ları ve worker'ı kaydetmek

Bir job gönderildiğinde **serileştirilir** (`Serialize`'dır), `NAME`'iyle etiketlenir ve queue'ya
konur. Worker, doğru tipi seçmek ve onu geri **deserialize** etmek için adı okur — bu yüzden `NAME`,
job'ların arasında benzersiz olmalıdır ve bir job'un alanlarını değiştirmek, zaten queue'ya alınmış
herhangi bir şey için bir format değişikliğidir. Worker'ın her adı nasıl route edeceğini bilmesi
için her job tipini listeleyen bir `Worker` kur ve onu `QueueProvider`'a geçir:

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

Bir handler'dan, queue'ya `ctx.queue()` (ya da `Inject<SharedQueue>`) ile eriş ve gönder:

```rust
#[post("/signup")]
async fn signup(ctx: Ctx, db: Db, /* ... */) -> Result<Redirect> {
    let user = /* kullanıcı oluştur */;
    ctx.queue()?.dispatch(SendWelcome { user_id: user.id }).await?;
    Ok(Redirect::to("/dashboard"))
}
```

Worker'ın bilmediği bir job tipi deserialize edilemez: worker, yanlış kodu çalıştırmak yerine net bir
"`<name>` job'u için kayıtlı handler yok" hatası bildirir. Yani yeni bir job tipi eklemek, bir
`.register::<T>()` satırı eklemek demektir.

## Worker'ı çalıştırmak

Queue'ya alınmış job'ları konsol komutuyla işle (`QueueProvider` tarafından kaydedilir):

```bash
cargo run -- queue:work        # Ctrl-C'ye kadar kuyruğu boşaltır
```

Geliştirmede onu `cargo run`'ın yanında çalıştırabilirsin; üretimde onu ayrı bir süreç olarak
çalıştır. Worker ve web süreci aynı `App::builder()`'ı paylaşır; böylece bir job'un `handle`'ı, bir
request handler'ıyla tam olarak aynı bağlama sahiptir — veritabanı, mailer, servislerin.

## Bugün gelen şey

Yerleşik queue **süreç içidir**: gönderilen job'lar, `queue:work`'ün boşalttığı bir kanal üzerinden
gider. Job'lar request'i aşar ama **bir süreç yeniden başlatmasını değil**. Dayanıklı bir
veritabanı/Redis backend'i daha sonra aynı `Queue` trait'inin arkasına takılır — job kodun değişmez.
Şimdilik, bir yeniden başlatmada kaybetmeyi göze alamayacağın job için onu kullanma.

> `rustali make:job <Name>`, bir job oluşturur.
