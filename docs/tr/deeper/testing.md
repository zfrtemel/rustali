# Test

Rustali, süreç içi bir test istemcisiyle gelir. Gerçek uygulamanı çalıştırır — `main` ile aynı
provider'lar, route'lar ve middleware — ama hiçbir sokete bağlanmaz; böylece testler hızlı ve
belirleyici olur. Hata işleme, session'lar ve CSRF'in hepsi tam olarak üretimdeki gibi davranır.

## Kurulum

dev-dependencies'e `test` özelliğini ekle:

```toml
[dev-dependencies]
rustali = { version = "0.1", features = ["test"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Testlerin `main`'in kurduğu uygulamanın aynısını kurabilmesi için, bir kütüphane hedefinden bir
`app()` builder'ı dışa aç ([Proje Yapısı](../getting-started/structure)'na bak).

## Bir test yazmak

Uygulamayı kur, router'ını bir `TestClient` ile sar ve response'lar üzerinde doğrulamalar yap:

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

`test_config()`, teste uygun bir yapılandırma verir: `test` ortamı, debug açık ve sabit bir uygulama
anahtarı. Veritabanı ayarla**maz** — testin DB'ye dokunuyorsa birini kendin ekle.

## Veritabanını izole etmek

Veritabanına dokunan bir test için her teste kendi tek kullanımlık veritabanını ver; böylece testler
birbirinin satırlarını asla görmez. Yapılandırma bölümleri `Config::extra`'da tutulur; dolayısıyla
uygulamayı kurmadan önce `database` bölümünü bellek içi bir SQLite'a yönlendirebilirsin:

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

Bellek içi bir SQLite veritabanı her bağlantı için sıfırdan oluşturulur; böylece her test boş başlar.
Tıpkı `main`'in yaptığı gibi, testin başında migration'larını çalıştır (ya da şemayı oluştur). Bir
test içinde verinin bağlantılar arasında kalıcı olması gerekiyorsa bunun yerine tek kullanımlık bir
dosyaya yönlendir.

## İstemci

Bir `TestClient`, kurulmuş bir `Router`'ı sarar ve çağrılar arasında bir **çerez kavanozu** tutar:
her request o ana kadar saklanmış çerezleri gönderir ve response'taki her `Set-Cookie`'yi yakalar.
Yani bir request'teki giriş, sonraki request tarafından görülür — session'lar, flash mesajları ve
CSRF'in hepsi, tıpkı bir tarayıcının yapacağı gibi bir çağrı dizisi boyunca çalışır. (İstemci bilerek
`Clone` değildir: kavanoz, tek bir istemciye bağlı paylaşılan durumdur.)

| Metot | Gönderdiği |
|---|---|
| `get(path)` / `delete(path)` | bir GET / DELETE |
| `post_json(path, &body)` / `patch_json(path, &body)` | JSON gövdeli bir request |
| `post_form(path, &[(k, v)])` | URL kodlu bir form |
| `send(method, path, body, headers)` | istediğin gibi bir request |

Her request metodu bir `TestResponse` döndürür.

## İddialar

Her `assert_*` response'u geri döndürür; böylece zincirlenebilirler:

| İddia | Kontrol ettiği |
|---|---|
| `assert_ok()` | durum 200 |
| `assert_status(code)` | tam bir durum |
| `assert_redirect(Some("/to"))` | bir 3xx (isteğe bağlı olarak bir konuma) |
| `assert_see(text)` / `assert_dont_see(text)` | gövde verilen alt dizeyi içerir / içermez |
| `assert_json_has(pointer)` | bir RFC 6901 işaretçisinde bir JSON değeri var |
| `assert_json_at(pointer, &value)` | o konumdaki JSON değeri eşit |
| `assert_header(name, Some(value))` | bir başlık mevcut / verilen değere eşit |

Bir doğrulama yerine ham değerlere ihtiyacın olduğunda `TestResponse` bunları doğrudan sunar:
`text()` (gövdeyi `String` olarak), `json()` (gövdeyi `serde_json::Value`'ya ayrıştırılmış) ve
`header(name)` (`Option<String>`). Aşağıdaki CSRF örneği, token'ı render edilen formdan çekmek için
`text()` kullanıyor.

## Örnek: bir form akışı

Bu örnek, blog'un entegrasyon testini birebir yansıtır — CSRF ve validation dahil tüm SSR yolunu,
süreç içinde çalıştırır:

```rust
let client = TestClient::new(router);

// form bir CSRF token'ıyla render edilir
let home = client.get("/").await;
home.assert_ok().assert_see("No posts yet.");
let token = extract_token(&home.text());

// geçersiz bir gönderim doğrulamada takılır + eski girdiyle geri yönlendirir
client.post_form("/posts", &[("_token", &token), ("title", "Hi"), ("body", "")])
    .await
    .assert_redirect(Some("/"));
client.get("/").await.assert_see("between 3 and 200").assert_see("value=\"Hi\"");

// CSRF token'ı eksik gönderim reddedilir
client.post_form("/posts", &[("title", "x"), ("body", "y")])
    .await
    .assert_status(rustali::http::StatusCode::FORBIDDEN);
```

Tüm stack'i çalıştırdığı için böyle bir test; routing'i, ORM'i, validation'ı, session'ları ve
CSRF'i bir arada sınar.

## Kimliği doğrulanmış route'ları test etmek

Bir `acting_as` kısayolu yok — zaten pek ihtiyacın da olmaz, çünkü çerez kavanozu sayesinde tıpkı
bir kullanıcı gibi giriş yapabilirsin. Giriş route'una post yap; sonraki request'te session çerezi
korunan route'a kendiliğinden taşınsın:

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

Böylece kimliği doğrulanmış bir durumu taklit etmek yerine gerçek giriş yolunu test etmiş olursun —
parola kontrolü, session oluşturma ve korunan route'taki guard dahil.

> `rustali make:test <name>`, bir test oluşturur.
