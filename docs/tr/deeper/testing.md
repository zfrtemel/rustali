# Test

Rustali, süreç içi bir test istemcisi getirir. Gerçek uygulamanı sürer — `main` ile aynı
provider'lar, route'lar ve middleware — bir soket bağlamadan; böylece testler hızlı ve
deterministiktir. Hata pazarlığı, session'lar ve CSRF'in hepsi tam olarak üretimdeki gibi davranır.

## Kurulum

dev-dependencies'e `test` özelliğini ekle:

```toml
[dev-dependencies]
rustali = { version = "0.1", features = ["test"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Testlerin `main`'in kurduğu aynı uygulamayı kurması için, bir kütüphane hedefinden bir `app()`
builder'ı göster ([Proje Yapısı](../getting-started/structure)'na bak).

## Bir test yazmak

Uygulamayı kur, router'ını bir `TestClient`'a sar ve response'lar üzerinde iddiada bulun:

```rust
use rustali::test::{TestClient, test_config};
use rustali::prelude::*;

#[tokio::test]
async fn home_works() {
    let (_ctx, router) = App::builder()
        .config(test_config())
        .controllers()
        .build()
        .await
        .unwrap();
    let client = TestClient::new(router);

    client.get("/").await
        .assert_ok()
        .assert_see("Welcome");
}
```

`test_config()`, test dostu bir yapılandırma verir: `test` ortamı, debug açık ve sabit bir uygulama
anahtarı. Bir veritabanı ayarla**maz** — testin DB'ye dokunuyorsa bir tane ekle.

## Veritabanını izole etmek

Veritabanına ulaşan bir test için, her teste kendi tek kullanımlık veritabanını ver; böylece testler
asla birbirinin satırlarını görmez. Bir yapılandırma bölümü `Config::extra`'da saklanır; bu yüzden
uygulamayı kurmadan önce `database` bölümünü bir bellek içi SQLite'a yönlendirebilirsin:

```rust
fn test_app_config() -> Config {
    let mut config = test_config();
    config.extra.insert(
        "database".into(),
        toml::toml! { url = "sqlite::memory:" }.into(),
    );
    config
}
```

Bir bellek içi SQLite veritabanı bağlantı başına taze oluşturulur; böylece her test boş başlar.
Migration'larını çalıştır (ya da şemayı oluştur) testin başında, tam olarak `main`'in yapacağı gibi.
Bir test içinde verinin bağlantılar arasında kalıcı olması gerekiyorsa bunun yerine tek kullanımlık
bir dosyaya yönlendir.

## İstemci

Bir `TestClient`, kurulan bir `Router`'ı sarar ve çağrılar arasında bir **çerez kavanozu** tutar:
her request şimdiye kadar saklanan çerezleri gönderir ve response'taki herhangi bir `Set-Cookie`'yi
yakalar. Yani bir request'teki bir giriş, sonraki tarafından görülür — session'lar, flash mesajları ve
CSRF'in hepsi, tam olarak bir tarayıcının süreceği gibi bir çağrı dizisi boyunca çalışır. (İstemci
bilinçli olarak `Clone` değildir: kavanoz, bir istemciye kapsamlanmış paylaşılan durumdur.)

| Metot | Gönderdiği |
|---|---|
| `get(path)` / `delete(path)` | bir GET / DELETE |
| `post_json(path, &body)` / `patch_json(path, &body)` | bir JSON gövdesi |
| `post_form(path, &[(k, v)])` | bir URL kodlu form |
| `send(method, path, body, headers)` | rastgele bir request |

Her request metodu bir `TestResponse` döndürür.

## İddialar

Her `assert_*` response'u döndürür; böylece zincirlenirler:

| İddia | Kontrol ettiği |
|---|---|
| `assert_ok()` | durum 200 |
| `assert_status(code)` | tam bir durum |
| `assert_redirect(Some("/to"))` | bir 3xx (isteğe bağlı olarak bir konuma) |
| `assert_see(text)` / `assert_dont_see(text)` | gövde bir alt dize içerir / atlar |
| `assert_json_has(pointer)` | bir RFC 6901 işaretçisinde bir JSON değeri var |
| `assert_json_at(pointer, &value)` | JSON değeri eşit |
| `assert_header(name, Some(value))` | bir başlık mevcut / eşit |

Bir iddia yerine ham değerlere ihtiyacın olduğunda, `TestResponse` onları doğrudan gösterir:
`text()` (gövde bir `String` olarak), `json()` (gövde bir `serde_json::Value`'ya ayrıştırılmış) ve
`header(name)` (`Option<String>`). Aşağıdaki CSRF örneği, token'ı render edilen formdan çekmek için
`text()` kullanır.

## Örnek: bir form akışı

Bu, blog'un entegrasyon testini yansıtır — tüm SSR yolunu, CSRF ve validation dahil, süreç içinde
sürer:

```rust
let client = TestClient::new(router);

// form bir CSRF token'ıyla render edilir
let home = client.get("/").await;
home.assert_ok().assert_see("No posts yet.");
let token = extract_token(&home.text());

// geçersiz bir gönderim hatalar + eski girdiyle geri yönlendirir
client.post_form("/posts", &[("_token", &token), ("title", "Hi"), ("body", "")])
    .await
    .assert_redirect(Some("/"));
client.get("/").await.assert_see("between 3 and 200").assert_see("value=\"Hi\"");

// eksik bir CSRF token'ı reddedilir
client.post_form("/posts", &[("title", "x"), ("body", "y")])
    .await
    .assert_status(rustali::http::StatusCode::FORBIDDEN);
```

Tüm stack'i çalıştırdığı için, böyle bir test routing'i, ORM'i, validation'ı, session'ları ve
CSRF'i birlikte test eder.

## Kimliği doğrulanmış route'ları test etmek

Bir `acting_as` kısayolu yok — ve nadiren birine ihtiyacın var, çünkü çerez kavanozu bir kullanıcının
yaptığı gibi giriş yapmana izin verir. Giriş route'una post yap, sonra session çerezi korunan bir
route'a sonraki request'te binsin:

```rust
let client = TestClient::new(router);

// giriş yap (yanıtın Set-Cookie'si kavanozda saklanır)
client
    .post_form("/login", &[("email", "ada@example.com"), ("password", "secret")])
    .await
    .assert_redirect(Some("/dashboard"));

// sonraki istek kimliği doğrulanmıştır, çünkü kavanoz oturum çerezini yeniden oynatır
client.get("/dashboard").await.assert_ok().assert_see("Welcome back");
```

Bu, kimliği doğrulanmış bir durumu taklit etmek yerine gerçek giriş yolunu test eder — parola
kontrolü, session oluşturma, korunan route'taki guard.

> `rustali make:test <name>`, bir test oluşturur.
