# Cache

Cache, iki deposu olan tek bir async trait'tir — süreç içi bir bellek deposu (varsayılan) ve isteğe
bağlı bir Redis deposu. En sık başvuracağın metot `remember`: tek çağrıda getir, yoksa hesapla ve
sakla.

## İki trait, bir cache

Cache iki trait'e ayrılır; hangisinin ne işe yaradığını bilmek tüm API'yi açıklamaya yeter:

- **`Cache`**, düşük seviyeli depo sözleşmesidir: **opak baytları** taşır (`get_bytes`, `put_bytes`,
  `forget`, `flush`). Her backend — bellek olsun, Redis olsun — bunu uygular.
- **`CacheExt`**, her `Cache` için **blanket olarak uygulanan** tipli ve ergonomik katmandır.
  `get::<T>`, `put`, `forever`, `remember` ve `has` metotlarını ekler, JSON serileştirmeyi kendi
  halleder; böylece baytlarla değil, kendi tiplerinle çalışırsın.

Neredeyse her zaman `CacheExt` metotlarını çağırırsın. Bu ayrım sayesinde yeni bir backend'in
yalnızca baytları doğru taşıması yeter; tipli yardımcıların hepsi hazır gelir.

## Kurulum

`CacheProvider` ekle; onu `.env`'de yapılandır:

```rust
App::builder().provider(CacheProvider).controllers().run().await
```

```bash
# .env
CACHE_DRIVER=memory                 # memory | redis
CACHE_URL=redis://127.0.0.1:6379    # CACHE_DRIVER=redis olduğunda kullanılır
```

`redis` driver'ı için `redis` Cargo özelliği gerekir. Bu özellik olmadan `driver = "redis"` istemek
sessizce başka bir depoya düşmez, net bir başlangıç hatası verir; böylece yanlış yapılandırılmış bir
üretim deployment'ı, sessizce süreç içinde cache'lemek yerine erkenden ve açıkça patlar.

## Kullanmak

Cache'e `ctx.cache()` ile eriş; bu, klonlayıp task'lara taşıyabileceğin bir `SharedCache`
(`Arc<dyn Cache>`) döndürür. Değerler JSON olarak serileştirilir; dolayısıyla `Serialize +
DeserializeOwned` olan her tip çalışır.

```rust
use std::time::Duration;

#[get("/stats")]
async fn stats(ctx: Ctx, db: Db) -> Result<Json<Stats>> {
    let cache = ctx.cache()?;
    let stats = cache
        .remember("stats", Duration::from_secs(60), || async {
            compute_stats(&db).await           // yalnızca önbellek kaçırmasında çalışır
        })
        .await?;
    Ok(Json(stats))
}
```

### Metotlar

| Metot | Yaptığı |
|---|---|
| `remember(key, ttl, f)` | Cache'teki değeri döndürür; yoksa `f` ile hesaplar, `ttl` boyunca saklar ve döndürür. |
| `get::<T>(key)` | `Option<T>`. |
| `put(key, &value, ttl)` | Bir sona erme süresiyle saklar. |
| `forever(key, &value)` | Sona erme süresi olmadan saklar. |
| `has(key)` | `bool`. |
| `forget(key)` | Bir anahtarı siler. |
| `flush()` | Her şeyi siler. |

```rust
let cache = ctx.cache()?;
cache.put("user:42", &user, Duration::from_secs(300)).await?;
let hit: Option<User> = cache.get("user:42").await?;
cache.forget("user:42").await?;
```

## Derinlemesine remember

İşin yükünü `remember` çeker. Verdiğin closure **yalnızca cache'te değer bulunamayınca** çalışır ve
bir `Result<T>` döndürmelidir — handler'larında kullandığın `Result`'ın aynısı — böylece içindeki
`?`, hataları doğrudan `remember` üzerinden dışarı taşır:

```rust
let user = cache
    .remember(&format!("user:{id}"), Duration::from_secs(300), || async {
        // yalnızca anahtar yok ya da sona ermiş olduğunda çalışır
        User::find_or_404(&db, id).await       // -> Result<User>
    })
    .await?;
```

Bilmeye değer birkaç nokta:

- Closure bir `FnOnce`'tır ve bir future döndürür; bu yüzden ihtiyacın olan kalıp `|| async { ... }`,
  `async || ...` değil.
- Hesaplama adımı başarısız olursa hiçbir şey cache'e yazılmaz ve hata döndürülür; bir sonraki çağrı
  yeniden dener.
- Saklanan tip `Serialize + DeserializeOwned` olmalı (değer JSON'a dönüp geri çözülür).

## Geçersiz kılma (invalidation)

Otomatik geçersiz kılma yok — cache'teki bir değerin ne zaman bayatladığına sen karar verirsin.
Yaygın kalıp, değeri bayatlatan yazma işleminden hemen sonra o anahtarı `forget` etmektir:

```rust
// kullanıcıyı güncelledikten sonra önbelleğe alınmış kopyayı düşür
user.update(&db).await?;
cache.forget(&format!("user:{id}")).await?;
```

Belirli aralıklarla tazelenmesi gereken değerler için elle geçersiz kılma yerine `remember`'da kısa
bir TTL kullan — hem daha basit, hem kendi kendini toparlayan bir yaklaşım.

## Depolar

- **Bellek** (varsayılan) — sürece özel bir map. Girişler **lazy** biçimde (erişim anında kontrol
  edilerek) ve arada bir yapılan temizlik sırasında expire olur; dolayısıyla yönetmen gereken bir
  arka plan thread'i yoktur. Altyapı gerektirmez, geliştirme ve tek düğümlü uygulamalar için biçilmiş
  kaftandır. Örnekler arasında **paylaşılmaz** ve süreç yeniden başladığında sıfırlanır.
- **Redis** (`redis` özelliği, `driver = "redis"`) — tüm örnekler arasında paylaşılır ve yeniden
  başlatmaları atlatır. Birden fazla süreç çalıştırdığında ya da cache'in bir deployment'tan daha
  uzun yaşamasını istediğinde bunu kullan.

İkisi de aynı `Cache` trait'inin arkasında durur; bu yüzden depo değiştirmek sadece bir yapılandırma
değişikliğidir, kodun hiç değişmez. Bellek deposu sürece özel olduğundan, bir düğümde cache'lenmiş
bir değer başka bir düğümde yoktur — bu, `remember` arkasındaki türetilmiş veriler için sorun değil,
ama örnekler arasında tutarlı olması gereken hiçbir şeyde buna güvenme (onun için Redis kullan).

## Ne zaman cache'e alınır

Cache, **türetilmiş, yeniden hesaplanabilir** veriler içindir: pahalı toplamlar, render edilmiş
parçalar, yavaş bir yukarı akış çağrısının sonuçları. Cache bir veritabanı değildir — yeniden
hesaplayamayacağın her şey gerçek bir depoda durmalıdır. `remember`'ı şöyle düşün: bir gerçek
kaynağın önündeki hızlı yol, asla gerçek kaynağın kendisi değil.
