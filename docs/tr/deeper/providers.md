# Servis Konteyneri & Provider'lar

Rustali'nin açık, tiple indekslenen bir servis konteyneri var. Servisleri başlangıçta **tipe** göre
kaydeder, request anında yine tipe göre bulursun. Dizeyle bağlama ya da otomatik bağlama sihri yok.

## Bir servis kaydetmek

En basit yol `provide`: builder'a hazır bir değer verirsin.

```rust
#[derive(Clone)]
struct Clock { /* ... */ }

App::builder()
    .provide(Clock::new())
    .controllers()
```

## Bir servis enjekte etmek

Bir handler içinde `Inject<T>` ile iste; `Inject<T>` de `T`'ye deref olur:

```rust
#[get("/now")]
async fn now(clock: Inject<Clock>) -> String {
    clock.iso8601()        // Inject<Clock>, Clock'a deref olur
}
```

Ya da elinde bir bağlam olan her yerde servise bağlam üzerinden eriş:

```rust
let clock = ctx.service::<Clock>()?;     // Arc<Clock> ya da kayıtlı değilse Err
```

Kimsenin kaydetmediği bir servisi istersen, tipin adını veren net bir hata alırsın; böylece
unutulmuş bir provider sessizce değil, açıkça patlar.

### Servisler paylaşılır

Kaydettiğin her değer bir `Arc`'a sarılır ve paylaşılan referans olarak verilir: `Inject<T>`
bir `Arc<T>` tutar ve `T`'ye deref olur; `ctx.service::<T>()` ise doğrudan `Arc<T>` döndürür. Yani bir
servis **bir kez kaydedilir ve** tüm request'ler arasında **paylaşılır** — request başına ayrı bir kopya
değil, tek bir `Clock`, tek bir veritabanı havuzu, tek bir HTTP istemcisi olur. Bunun iki sonucu var:

- Bir servis `Send + Sync + 'static` olmalı (aynı anda birçok task ona dokunur).
- Paylaşımı ucuz olmalı ve yerinde değiştirilmemeli. İç değişebilirlik için olağan araçları kullan
  (`Mutex`, `RwLock`, atomikler ya da kendi kilitlemesini yöneten bir bağlantı havuzu).

Kayıt **tipe göre** yapılır: tip başına tek bir değer. Aynı tipten ikinci bir değer kaydedersen
ilkinin yerini alır. Aynı temel tipten iki ayrı değere ihtiyacın varsa, her birini ayrı bir newtype'a sar.

## Provider'lar

Bir **provider**, bir paketin başlangıç kancasıdır. Kendi yapılandırma bölümünü okur ve sağladığı
şeyi kaydeder. Framework'ün kendi alt sistemleri de birer provider'dır (`DatabaseProvider`,
`SessionProvider`, `CacheProvider`, `MailProvider`, `AuthProvider`, `QueueProvider`); kendi
provider'ını da yazabilirsin.

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

Provider'lar, yapılandırma yüklendikten sonra ama route'lar kurulmadan önce, **eklendikleri sırayla**
çalışır. Bu sıra, güvenebileceğin bir sözleşmedir: sonraki bir provider, daha önce çalışan bir
provider'ın servislerine `registry.require::<T>()` ile kayıt defterinden çekerek bağımlı olabilir:

```rust
fn register<'a>(&'a self, registry: &'a mut Registry<'_>) -> RegisterFuture<'a> {
    Box::pin(async move {
        let db = registry.require::<Database>()?;   // daha önce DatabaseProvider tarafından kaydedildi
        registry.insert(InvoiceStore::new(db));
        Ok(())
    })
}
```

Bağımlılık yoksa `require` hata verir; böylece sırası yanlış bir kayıt, ileride kafa karıştıran
bir `None` olarak değil, net bir başlangıç hatası olarak ortaya çıkar. `register` async'tir, çünkü
kurulum çoğu zaman bir yere bağlanmak demektir — bir veritabanı havuzu, bir cache, bir e-posta taşıyıcısı.

### Yapılandırma bölümü

Alışılmış yaklaşım, provider başına bir yapılandırma bölümüdür ve bu `section_or_default` ile okunur:

```rust
let config: PaymentsConfig = registry.config().section_or_default("payments")?;
```

`payments` bölümü yoksa `section_or_default`, `PaymentsConfig::default()`'a düşer; böylece bir
provider hiç ayar yapmadan çalışır ve yalnızca gerektiğinde yapılandırılır. Yapılandırma tipi
sıradan bir `#[derive(Deserialize, Default)]` struct'tır. Böyle özel bir bölümü, `.env`'deki
`APP__PAYMENTS__*` anahtarlarıyla doldurursun
([Yapılandırma](../getting-started/configuration#ozel-ayarlar)'ya bak).

### Komut kaydetmek

Bir provider, `registry.command(...)` ile konsol komutları da kaydedebilir — `migrate` ve
`queue:work` komutları da provider'ları tarafından tam olarak böyle eklenir. [Konsol Komutları](console)'na bak.

## Request kapsamlı veri

Konteyner, paylaşılan ve uzun ömürlü servisleri tutar. Request başına değerler için ya middleware
üzerinden request uzantılarını kullan ya da veriyi handler'dan aşağıya doğru geçir — konteyner,
request başına durum için değil, havuzlar ve istemciler gibi şeyler içindir.

## Neden açık

Bu tercih bilinçli: kaydetmek için bir satır, enjekte etmek için bir tip yazarsın; karşılığında
çalışma zamanı yansıması ve sihirli çözümleme sırası olmaz, derleyici de her servis tipini denetler.
Eksik bir bağımlılık, çalışma zamanında çıkan bir muamma değil, adı belli olan bir hatadır.
