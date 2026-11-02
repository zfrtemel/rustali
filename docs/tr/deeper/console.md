# Konsol Komutları

Uygulama binary'n aynı zamanda konsoludur. Argümansız sunar; bir komutla bunun yerine onu
çalıştırır. `run()`'ın yaptığı budur:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    App::builder().provider(DatabaseProvider).controllers().run().await
}
```

```bash
cargo run                      # sun
cargo run -- routes            # `routes` komutunu çalıştır
cargo run -- migrate           # `migrate` komutunu çalıştır
cargo run -- help              # kullanılabilir komutları listele
```

Binary ilk argümana göre dağıtım yapar: argüman yok (ya da `serve`) sunucuyu çalıştırır, `help`
komutları listeler ve diğer herhangi bir sözcük kayıtlı komutlar arasında aranır — bilinmeyen bir
ad net bir hatadır. Yani binary'nin anladığı komut kümesi, tam olarak provider'larının ve
`.command(...)` çağrılarının kaydettiği şeydir — fazlası değil, gizli bir şey yok.

## Yerleşik komutlar

Hangi komutların var olduğu, hangi provider'ları eklediğine bağlıdır:

| Komut | Nereden | Yaptığı |
|---|---|---|
| `serve` | her zaman | HTTP sunucusunu başlat (varsayılan). |
| `routes` | her zaman | Controller route'larını middleware'leriyle listele. |
| `help` | her zaman | Komutları listele. |
| `migrate`, `migrate:diff`, `migrate:status`, `migrate:rollback`, `migrate:fresh`, `migrate:refresh`, `migrate:reset` | `.migrations::<M>()` | Veritabanı migration'ları. |
| `queue:work` | `QueueProvider` | Queue'ya alınmış job'ları işle. |

`help`, her kayıtlı komutu tek satırlık `about`'uyla listeler; böylece belirli bir derlemenin neyi
desteklediğini görmenin en hızlı yolu onu çalıştırmaktır:

```bash
cargo run -- help
```

## Bir komut yazmak

`Command`'ı uygula: bir ad, bir satırlık bir açıklama ve `AppContext`'i ve kalan argümanları alan
async bir `run`.

```rust
use rustali::prelude::*;
use rustali::{Command, CommandFuture};

struct SendReport;

impl Command for SendReport {
    fn name(&self) -> &'static str { "report:send" }
    fn about(&self) -> &'static str { "Email the daily report" }

    fn run<'a>(&'a self, ctx: &'a AppContext, args: Vec<String>) -> CommandFuture<'a> {
        Box::pin(async move {
            let db = ctx.db()?;
            // ... raporu oluştur ve gönder ...
            println!("sent to {} recipients", args.len());
            Ok(())
        })
    }
}
```

Onu builder'da (ya da bir provider'dan `registry.command(...)` ile) kaydet:

```rust
App::builder()
    .command(SendReport)
    .controllers()
    .run()
    .await
```

```bash
cargo run -- report:send alice@example.com bob@example.com
```

### Argümanlar

`run`, komut adından sonraki her şeyi bir `Vec<String>` olarak alır — binary adı ve komut sözcüğü
zaten çıkarılmıştır. `cargo run -- report:send a@x b@x` için, `args` `["a@x", "b@x"]`'tir. Onu kendin
ayrıştır (ya da `clap` gibi bir crate'e ver); framework komutlarına bir bayrak söz dizimi dayatmaz.

### Hatalar ve çıkış kodları

`run`, `Result<()>` döndürür ve o sonuç `main`'e kadar akar. `main`, `-> Result<()>` olduğu için,
bir `Err` döndürmek onu yazdırır ve sıfır olmayan bir durumla çıkar — yani başarısız bir komut,
herhangi bir elle `std::process::exit` olmadan shell'i de (ve CI'nı da) başarısız kılar. `?`'yi
serbestçe kullan:

```rust
fn run<'a>(&'a self, ctx: &'a AppContext, args: Vec<String>) -> CommandFuture<'a> {
    Box::pin(async move {
        let to = args.first().ok_or_else(|| Error::bad_request("usage: report:send <email>"))?;
        let db = ctx.db()?;
        // ...
        Ok(())
    })
}
```

### Tam bağlam

Bir komut aynı `App::builder()` aracılığıyla çalıştığı için, tam bağlama sahiptir — veritabanı,
cache, mailer, servislerin — tam olarak bir request handler'ının sahip olduğu gibi. Provider'lar
komuttan önce çalışır; böylece kaydettikleri her şey `ctx` aracılığıyla kullanılabilir. Bir
handler'dan tek farkı giriş noktasıdır: HTTP request'i yoktur, yalnızca `ctx` ve `args`.

## İskeletleme

`rustali make:controller`, `make:model`, `make:migration`, `make:job`, `make:mail`,
`make:middleware`, `make:policy` ve `make:test`, bağımsız `rustali` CLI'sinin (uygulama binary'sinin
değil) komutlarıdır — kaynak dosyalar üretirler. [Proje Yapısı](../getting-started/structure)'na bak.
