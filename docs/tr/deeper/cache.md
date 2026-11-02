# Önbellek

Önbellek, iki depolu tek bir async trait'tir — bir süreç içi bellek deposu (varsayılan) ve isteğe
bağlı bir Redis deposu. En çok başvuracağın metot `remember`'dır: tek bir çağrıda
getir-ya-hesapla-ve-sakla.

## İki trait, bir önbellek

Önbellek iki trait'e ayrılır ve hangisinin hangisi olduğunu bilmek tüm API'yi açıklar:

- **`Cache`**, düşük seviyeli depo sözleşmesidir: **opak baytlar** taşır (`get_bytes`, `put_bytes`,
  `forget`, `flush`). Her arka uç — bellek, Redis — bunu uygular.
- **`CacheExt`**, her `Cache` üzerinde **blanket olarak uygulanan** tipli, ergonomik katmandır.
  `get::<T>`, `put`, `forever`, `remember` ve `has`'i ekler, JSON serileştirmeyi işler; böylece
  baytlarla değil, kendi tiplerinle çalışırsın.

Neredeyse her zaman `CacheExt` metotlarını çağırırsın. Bölünme, yeni bir arka ucun yalnızca
baytları doğru taşıması gerektiği anlamına gelir; her tipli yardımcı bedava gelir.

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

`redis` sürücüsü `redis` Cargo özelliğine ihtiyaç duyar. Onsuz, `driver = "redis"` istemek sessiz
bir geri dönüş değil, net bir başlangıç hatasıdır; böylece yanlış yapılandırılmış bir üretim
dağıtımı, sessizce süreç içinde önbelleğe almak yerine hızlı başarısız olur.

## Kullanmak

Önbelleğe `ctx.cache()` ile eriş; bu, klonlayıp task'lara taşıyabileceğin bir `SharedCache`
(`Arc<dyn Cache>`) döndürür. Değerler JSON ile serileştirilir; böylece herhangi bir
`Serialize + DeserializeOwned` tipi çalışır.

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
| `remember(key, ttl, f)` | Önbelleğe alınmış değeri döndür ya da `f` ile hesapla, `ttl` boyunca sakla, onu döndür. |
| `get::<T>(key)` | `Option<T>`. |
| `put(key, &value, ttl)` | Bir sona erme ile sakla. |
| `forever(key, &value)` | Sona erme olmadan sakla. |
| `has(key)` | `bool`. |
| `forget(key)` | Bir anahtarı kaldır. |
| `flush()` | Her şeyi kaldır. |

```rust
let cache = ctx.cache()?;
cache.put("user:42", &user, Duration::from_secs(300)).await?;
let hit: Option<User> = cache.get("user:42").await?;
cache.forget("user:42").await?;
```

## Derinlemesine remember

`remember` iş gücüdür. Closure'ı **yalnızca bir kaçırmada** çalışır ve bir `Result<T>` döndürmelidir
— handler'larının kullandığı aynı `Result` — böylece içindeki `?`, `remember`'dan doğrudan hataları
yayar:

```rust
let user = cache
    .remember(&format!("user:{id}"), Duration::from_secs(300), || async {
        // yalnızca anahtar yok ya da sona ermiş olduğunda çalışır
        User::find_or_404(&db, id).await       // -> Result<User>
    })
    .await?;
```

Bilmeye değer birkaç şey:

- Closure `FnOnce`'tır ve bir future döndürür; bu yüzden `|| async { ... }` şekli istediğin şeydir —
  `async || ...` değil.
- Hesaplama adımı başarısız olursa, hiçbir şey önbelleğe alınmaz ve hata döndürülür; sonraki çağrı
  tekrar dener.
- Saklanan tip `Serialize + DeserializeOwned` olmalıdır (JSON üzerinden gidip gelir).

## Geçersiz kılma (invalidation)

Otomatik geçersiz kılma yoktur — bir önbelleğe alınmış değerin ne zaman bayatladığına sen karar
verirsin. Yaygın desen, onu bayatlatan yazmadan hemen sonra bir anahtarı `forget` etmektir:

```rust
// kullanıcıyı güncelledikten sonra önbelleğe alınmış kopyayı düşür
user.update(&db).await?;
cache.forget(&format!("user:{id}")).await?;
```

Sabit bir kadansta yenilenmesi gereken değerler için, elle geçersiz kılma yerine `remember`'da kısa
bir TTL tercih et — daha basit ve kendi kendini iyileştiren.

## Depolar

- **Bellek** (varsayılan) — süreç yerel bir eşleme. Girişler **tembel** sona erer (erişimde kontrol
  edilir) ve ara sıra bir süpürme sırasında; bu yüzden yönetilecek bir arka plan iş parçacığı
  yoktur. Altyapı yok, geliştirme ve tek düğümlü uygulamalar için harika. Örnekler arasında
  **paylaşılmaz** ve süreç yeniden başladığında temizlenir.
- **Redis** (`redis` özelliği, `driver = "redis"`) — her örnek arasında paylaşılır ve yeniden
  başlatmalarda hayatta kalır. Birden fazla süreç çalıştırdığında ya da önbelleğin bir dağıtımdan
  daha uzun yaşamasını istediğinde onu kullan.

İkisi de aynı `Cache` trait'inin arkasında oturur; böylece depo değiştirmek bir yapılandırma
değişikliğidir; kodun taşınmaz. Bellek deposu süreç başına olduğu için, bir düğümde önbelleğe
alınan bir değer başka bir düğümde bir kaçırmadır — `remember` arkasındaki türetilmiş veriler için
uygun, ama örnekler arasında tutarlı olması gereken hiçbir şey için ona güvenme (bunun için Redis
kullan).

## Ne zaman önbelleğe alınır

Önbellek, **türetilmiş, yeniden hesaplanabilir** veri içindir: pahalı toplamlar, render edilmiş
parçalar, yavaş bir yukarı akış çağrısının sonuçları. Bir veritabanı değildir — yeniden
hesaplayamayacağın hiçbir şey gerçek bir depoda yaşamalıdır. `remember` için zihinsel model, "bir
gerçek kaynağın önünde hızlı bir yol"dur, asla gerçek kaynağın kendisi değil.
