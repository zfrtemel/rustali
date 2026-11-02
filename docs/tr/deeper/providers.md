# Servis Konteyneri & Provider'lar

Rustali'nin açık, tip-indeksli bir servis konteyneri vardır. Servisler başlangıçta **tipe** göre
kaydedilir ve request anında tipe göre aranır. Dize bağlama ve otomatik-bağlama sihri yoktur.

## Bir servis kaydetmek

En basit yol `provide`'dır — builder'a hazır bir değer ver:

```rust
#[derive(Clone)]
struct Clock { /* ... */ }

App::builder()
    .provide(Clock::new())
    .controllers()
```

## Bir servis enjekte etmek

Onu bir handler'da `Inject<T>` ile iste; o da `T`'ye deref olur:

```rust
#[get("/now")]
async fn now(clock: Inject<Clock>) -> String {
    clock.iso8601()        // Inject<Clock>, Clock'a deref olur
}
```

Ya da bir tanesine sahip olduğun her yerde bağlam aracılığıyla ona eriş:

```rust
let clock = ctx.service::<Clock>()?;     // Arc<Clock> ya da kayıtlı değilse Err
```

Kimsenin kaydetmediği bir servis, tipi adlandıran net bir hata verir; böylece unutulmuş bir
provider sessizce değil, yüksek sesle başarısız olur.

### Servisler paylaşılır

Kaydedilen her değer bir `Arc`'a sarılır ve paylaşılan referansla verilir: `Inject<T>` bir
`Arc<T>` tutar ve `T`'ye deref olur; `ctx.service::<T>()`, `Arc<T>`'yi doğrudan döndürür. Yani bir
servis **bir kez kaydedilir ve** her request tarafından **paylaşılır** — request başına bir kopya değil,
bir `Clock`, bir veritabanı havuzu, bir HTTP istemcisi vardır. İki sonuç:

- Bir servis `Send + Sync + 'static` olmalıdır (aynı anda birçok task'tan dokunulur).
- Paylaşması ucuz olmalı, yerinde değiştirilmemeli. İç değişebilirlik için olağan araçları kullan
  (`Mutex`, `RwLock`, atomik'ler ya da kendi kilitlemesini yöneten bir bağlantı havuzu).

Kayıt **tipe göredir**: tip başına bir değer. Aynı tipin ikinci bir değerini kaydetmek ilkinin
yerine geçer. Aynı altta yatan tipin ikisine ihtiyacın varsa, her birini ayrı bir newtype'a sar.

## Provider'lar

Bir **provider**, bir paketin başlangıç kancasıdır. Kendi yapılandırma bölümünü okur ve sahip
olduğu şeyi kaydeder. Framework'ün kendi alt sistemleri provider'dır (`DatabaseProvider`,
`SessionProvider`, `CacheProvider`, `MailProvider`, `AuthProvider`, `QueueProvider`) ve kendi
provider'ını yazabilirsin.

```rust
use rustali::prelude::*;

struct PaymentsProvider;

impl Provider for PaymentsProvider {
    fn name(&self) -> &'static str { "payments" }

    fn register<'a>(&'a self, registry: &'a mut Registry<'_>) -> RegisterFuture<'a> {
        Box::pin(async move {
            // `payments` yapılandırma bölümünü oku, bir istemci kur, kaydet
            let config: PaymentsConfig = registry.config().section_or_default("payments")?;
            registry.insert(PaymentsClient::connect(&config).await?);
            Ok(())
        })
    }
}

App::builder().provider(PaymentsProvider).controllers().run().await
```

### Yaşam döngüsü ve sıralama

Provider'lar, yapılandırma yüklendikten sonra ve route'lar kurulmadan önce, **eklendikleri sırayla**
çalışır. Bu sıralama, güvenebileceğin bir sözleşmedir: sonraki bir provider, erken bir provider'ın
servislerine `registry.require::<T>()` ile kayıt defterinden çekerek bağımlı olabilir:

```rust
fn register<'a>(&'a self, registry: &'a mut Registry<'_>) -> RegisterFuture<'a> {
    Box::pin(async move {
        let db = registry.require::<Database>()?;   // daha önce DatabaseProvider tarafından kaydedildi
        registry.insert(InvoiceStore::new(db));
        Ok(())
    })
}
```

`require`, bağımlılık eksikse hata verir; böylece sırasız bir kayıt, daha sonra kafa karıştırıcı
bir `None` değil, net bir başlangıç başarısızlığıdır. `register` async'tir çünkü kurulum genellikle
bağlanmak anlamına gelir — bir veritabanı havuzu, bir cache, bir e-posta taşıyıcısı.

### Yapılandırma bölümü

Konvansiyon, provider başına bir yapılandırma bölümüdür, `section_or_default` ile okunur:

```rust
let config: PaymentsConfig = registry.config().section_or_default("payments")?;
```

`section_or_default`, `payments` bölümü yoksa `PaymentsConfig::default()`'a fallback yapar; böylece bir
provider kutudan çıktığı gibi çalışır ve yalnızca gerektiğinde yapılandırılır. Yapılandırma tipi
sıradan bir `#[derive(Deserialize, Default)]` struct'tır. Bunun gibi özel bir bölüm, `.env`'den
`APP__PAYMENTS__*` anahtarlarıyla doldurulur
([Yapılandırma](../getting-started/configuration#ozel-ayarlar)'ya bak).

### Komut kaydetmek

Bir provider ayrıca `registry.command(...)` ile konsol komutları da kaydedebilir — `migrate` ve
`queue:work`'ün provider'ları tarafından eklenme şekli budur. [Konsol Komutları](console)'na bak.

## Request kapsamlı veri

Konteyner, paylaşılan, uzun ömürlü servisleri tutar. Request başına değerler için, middleware
aracılığıyla request uzantılarını kullan ya da veriyi handler'dan aşağıya geçir — konteyner, request
başına durum için değil, havuzlar ve istemciler gibi şeyler içindir.

## Neden açık

Takas bilinçlidir: kaydetmek için bir satır ve enjekte etmek için bir tip yazarsın ve karşılığında
çalışma zamanı yansıması, sihirli çözüm sırası olmaz ve derleyici her servis tipini denetler. Eksik
bir bağımlılık, çalışma zamanında bir gizem değil, adı olan bir hatadır.
