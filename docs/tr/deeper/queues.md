# Kuyruklar & İşler

Yavaş işi (e-posta, görüntü işleme, üçüncü taraf bir API çağırma) istek yolundan uzaklaştır. Onu bir
`Job` olarak tanımla, bir handler'dan gönder ve bir worker onu arka planda çalıştırsın.

## Bir iş tanımlamak

Bir iş, `Job`'ı uygulayan serileştirilebilir bir struct'tır. Kendini adlandırır, isteğe bağlı olarak
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
gecikmedir (üstel backoff daha sonra gelebilir). Yeniden denemelerini tüketen bir iş loglanır ve
düşürülür — henüz bir ölü-mektup deposu yok — bu yüzden yeniden denemeleri bir garanti değil, en iyi
çaba tamponu olarak gör.

### İşleri yeniden denemesi güvenli olacak şekilde yaz

Bir iş birden fazla kez çalışabileceği için (bir yeniden deneme ya da — daha sonra dayanıklı bir
arka uçla — en-az-bir-kez teslimat), önemli olduğu yerde `handle`'ı **idempotent** yap: aynı
e-postayı iki kez göndermeye, iki kez ücretlendirmeye ya da yinelenen bir satır eklemeye karşı
koru. Tekrar çalıştırılması güvenli bir iş, yeniden denendiğinde asla üzerinde çok düşünmek zorunda
olmadığın bir iştir.

## İşleri ve worker'ı kaydetmek

Bir iş gönderildiğinde **serileştirilir** (`Serialize`'dır), `NAME`'iyle etiketlenir ve kuyruğa
konur. Worker, doğru tipi seçmek ve onu geri **deserialize** etmek için adı okur — bu yüzden `NAME`,
işlerin arasında benzersiz olmalıdır ve bir işin alanlarını değiştirmek, zaten kuyruğa alınmış
herhangi bir şey için bir format değişikliğidir. Worker'ın her adı nasıl yönlendireceğini bilmesi
için her iş tipini listeleyen bir `Worker` kur ve onu `QueueProvider`'a geçir:

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

Bir handler'dan, kuyruğa `ctx.queue()` (ya da `Inject<SharedQueue>`) ile eriş ve gönder:

```rust
#[post("/signup")]
async fn signup(ctx: Ctx, db: Db, /* ... */) -> Result<Redirect> {
    let user = /* kullanıcı oluştur */;
    ctx.queue()?.dispatch(SendWelcome { user_id: user.id }).await?;
    Ok(Redirect::to("/dashboard"))
}
```

Worker'ın bilmediği bir iş tipi deserialize edilemez: worker, yanlış kodu çalıştırmak yerine net bir
"`<name>` işi için kayıtlı handler yok" hatası bildirir. Yani yeni bir iş tipi eklemek, bir
`.register::<T>()` satırı eklemek demektir.

## Worker'ı çalıştırmak

Kuyruğa alınmış işleri konsol komutuyla işle (`QueueProvider` tarafından kaydedilir):

```bash
cargo run -- queue:work        # Ctrl-C'ye kadar kuyruğu boşaltır
```

Geliştirmede onu `cargo run`'ın yanında çalıştırabilirsin; üretimde onu ayrı bir süreç olarak
çalıştır. Worker ve web süreci aynı `App::builder()`'ı paylaşır; böylece bir işin `handle`'ı, bir
istek handler'ıyla tam olarak aynı bağlama sahiptir — veritabanı, mailer, servislerin.

## Bugün gelen şey

Yerleşik kuyruk **süreç içidir**: gönderilen işler, `queue:work`'ün boşalttığı bir kanal üzerinden
gider. İşler isteği aşar ama **bir süreç yeniden başlatmasını değil**. Dayanıklı bir
veritabanı/Redis arka ucu daha sonra aynı `Queue` trait'inin arkasına takılır — iş kodun değişmez.
Şimdilik, bir yeniden başlatmada kaybetmeyi göze alamayacağın iş için onu kullanma.

> `rustali make:job <Name>`, bir iş oluşturur.
