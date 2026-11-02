# Konsol Komutları

Uygulama binary'n aynı zamanda konsoldur. Argümansız çalıştırınca sunucuyu açar; bir komut verince
onun yerine o komutu çalıştırır. `run()`'ın yaptığı tam olarak budur:

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

Binary, ilk argümana göre yönlendirme yapar: argüman yoksa (ya da `serve` ise) sunucuyu çalıştırır,
`help` komutları listeler, başka herhangi bir sözcüğü de kayıtlı komutlar arasında arar — bilinmeyen
bir ad net bir hata verir. Yani binary'nin anladığı komut kümesi, tam olarak provider'larının ve
`.command(...)` çağrılarının kaydettiği komutlardan ibarettir — ne fazlası, ne de gizlisi var.

## Yerleşik komutlar

Hangi komutların bulunacağı, hangi provider'ları eklediğine bağlıdır:

| Komut | Nereden | Yaptığı |
|---|---|---|
| `serve` | her zaman | HTTP sunucusunu başlatır (varsayılan). |
| `routes` | her zaman | Controller route'larını middleware'leriyle birlikte listeler. |
| `help` | her zaman | Komutları listeler. |
| `migrate`, `migrate:diff`, `migrate:status`, `migrate:rollback`, `migrate:fresh`, `migrate:refresh`, `migrate:reset` | `.migrations::<M>()` | Veritabanı migration'ları. |
| `queue:work` | `QueueProvider` | Queue'ya alınmış job'ları işler. |

`help`, kayıtlı her komutu tek satırlık `about` açıklamasıyla listeler; dolayısıyla belirli bir
derlemenin neleri desteklediğini görmenin en hızlı yolu bu komutu çalıştırmaktır:

```bash
cargo run -- help
```

## Bir komut yazmak

`Command`'ı uygula: bir ad, tek satırlık bir açıklama ve `AppContext` ile kalan argümanları alan
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

Komutu builder'da (ya da bir provider içinden `registry.command(...)` ile) kaydet:

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

`run`, komut adından sonraki her şeyi `Vec<String>` olarak alır — binary adı ve komut sözcüğü
önceden ayıklanmıştır. `cargo run -- report:send a@x b@x` için `args` değeri `["a@x", "b@x"]` olur.
Argümanları kendin ayrıştır (ya da `clap` gibi bir crate'e bırak); framework komutlara belirli bir
bayrak söz dizimi dayatmaz.

### Hatalar ve çıkış kodları

`run`, `Result<()>` döndürür ve bu sonuç `main`'e kadar yukarı taşınır. `main`, `-> Result<()>`
olduğu için bir `Err` döndürmek hatayı yazdırır ve sıfırdan farklı bir çıkış koduyla kapanır — yani
başarısız bir komut, sen elle `std::process::exit` çağırmadan hem shell'i hem CI'ı başarısız kılar.
`?`'yi gönül rahatlığıyla kullan:

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

Bir komut aynı `App::builder()` üzerinden çalıştığı için bağlamın tamamına sahiptir — veritabanı,
cache, mailer, servislerin — tıpkı bir request handler'ı gibi. Provider'lar komuttan önce çalışır;
böylece kaydettikleri her şeye `ctx` üzerinden erişilebilir. Bir handler'dan tek farkı giriş
noktasıdır: ortada HTTP request'i yoktur, yalnızca `ctx` ve `args` vardır.

## İskeletleme

`rustali make:controller`, `make:model`, `make:migration`, `make:job`, `make:mail`,
`make:middleware`, `make:policy` ve `make:test` komutları, uygulama binary'sine değil, bağımsız
`rustali` CLI'sine aittir — kaynak dosyalar üretirler. [Proje Yapısı](../getting-started/structure)'na bak.
