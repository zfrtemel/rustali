# Görünümler

Görünümler HTML render eder. Rustali varsayılan olarak **Askama** kullanır; o da şablonlarını
Rust'a derler ve derleme zamanında denetler — eksik bir değişken ya da bir şablondaki yazım
hatası, üretimde bir 500 değil, bir derleme hatasıdır. Hot reload ile çalışma zamanı şablonları
istersen, **Tera** arka ucu bir özelliğin arkasında mevcuttur.

## Askama (varsayılan)

Bir şablon, `Template`'i türeten ve bir şablon dosyası adlandıran bir struct'tır. Alanları,
şablonda kullanılabilir değişkenlerdir.

```rust
use rustali::prelude::*;   // `View` ve `Template`'i kapsama getirir

#[derive(Template)]
#[template(path = "posts/index.html")]
struct PostsIndex {
    posts: Vec<Post>,
    app_name: String,
}

#[get("/")]
async fn index(ctx: Ctx, db: Db) -> Result<View<PostsIndex>> {
    Ok(View::new(PostsIndex {
        posts: Post::query().order_by_desc(Post::ID).fetch_all(&db).await?,
        app_name: ctx.config().app.name.clone(),
    }))
}
```

Şablonlar varsayılan olarak `templates/` altında yaşar. Askama, crate kökünden bir `askama.toml`
okur:

```toml
# askama.toml
[general]
dirs = ["templates"]
```

```html
<!-- templates/posts/index.html -->
<h1>{{ app_name }}</h1>
{% for post in posts %}
  <article><h2>{{ post.title }}</h2><p>{{ post.body }}</p></article>
{% endfor %}
```

`View::new(page)`, bir `200 text/html` yanıtı döndürür. `with_status` ile farklı bir durum
ayarla — geçersiz bir formu yeniden render etmek için kullanışlı:

```rust
Ok(View::new(login_page).with_status(StatusCode::UNPROCESSABLE_ENTITY))
```

`View<T>` yalnızca bir `IntoResponse` olduğu için, `Result<View<T>>` döndüren bir handler da
diğerleri gibi otomatik hata render'ı alır.

### Bir şablon struct'ı bir görünüm modelidir

Şablon struct'ı tam olarak sayfanın ihtiyaç duyduğu veridir — fazlası değil. Varlıkları olduğu
gibi geçirmek yerine onu görünüm için şekillendir: biçimlendirilmiş bir tarihi, bir görünen adı,
bir bayrağı önceden hesapla. [Sayfalanmış](../database/queries#pagination) bir `Page<T>`'nin
`map`'i olduğundan, sayfalama sayılarını korurken satırları bir görünüm modeline
dönüştürebilirsin:

```rust
let page = Post::query().paginate(&db, 1, 20).await?
    .map(|post| PostRow { title: post.title, excerpt: truncate(&post.body, 140) });
```

Sunum mantığını şablon yerine handler'da (düz Rust) tutmak, şablonu basit ve mantığı test
edilebilir tutar.

## Düzenler (layout)

Askama şablon kalıtımını destekler. Bir temel düzen bloklar bildirir; sayfalar onu genişletir
ve doldurur.

```html
<!-- templates/layout.html -->
<!doctype html>
<html>
<head><title>{% block title %}{{ app_name }}{% endblock %}</title></head>
<body><main>{% block content %}{% endblock %}</main></body>
</html>
```

```html
<!-- templates/posts/index.html -->
{% extends "layout.html" %}
{% block title %}Posts{% endblock %}
{% block content %}
  {% for post in posts %}<article>{{ post.title }}</article>{% endfor %}
{% endblock %}
```

## Parçalar (partial)

Yeniden kullanılabilir bir parçayı başka bir şablona `{% include %}` ile çek. Parça, kendisini
dahil eden şablonla aynı bağlamı görür:

```html
<!-- templates/posts/_card.html -->
<article class="card"><h2>{{ post.title }}</h2><p>{{ post.body }}</p></article>
```

```html
<!-- templates/posts/index.html -->
{% for post in posts %}
  {% include "posts/_card.html" %}
{% endfor %}
```

## Bir bakışta şablon söz dizimi

Askama'nın söz dizimi Jinja benzeridir. En çok başvuracağın parçalar:

| Söz dizimi | Yaptığı |
|---|---|
| <code v-pre>{{ expr }}</code> | Bir değer yazdır (`.html`'de otomatik kaçışlı). |
| `{% if cond %} … {% else %} … {% endif %}` | Koşul. |
| `{% if let Some(x) = opt %} … {% endif %}` | Bir `Option`/deseni eşle (gerçek Rust desenleri). |
| `{% for item in items %} … {% endfor %}` | Döngü; `loop.index` 1 tabanlı konumu verir. |
| `{% extends "base.html" %}` / `{% block %}` | Düzen kalıtımı. |
| `{% include "partial.html" %}` | Başka bir şablonu satır içine al. |
| <code v-pre>{{ value\|filter }}</code> | Bir filtre uygula, örn. `\|safe`, `\|upper`. |

Şablonlar Rust'a derlendiği için içlerindeki ifadeler tip denetiminden geçer: var olmayan bir
metodu çağırmak ya da struct'ta olmayan bir alanı yazdırmak, `cargo build`'i başarısız kılar.

## Kaçış (escaping)

Askama, `.html` şablonlarında HTML'i otomatik kaçışlar. Güvenilir, önceden render edilmiş bir
dizeyi (bir CSRF gizli girdisi gibi) yaymak için onu `safe`'ten geçir:

```html
<form method="post" action="/posts">
  {{ csrf_field|safe }}
  ...
</form>
```

## Formlar: hatalar ve eski girdi

Bir [doğrulanmış](validation) form gönderimi başarısız olduğunda, hatalar ve gönderilen değerler
oturuma flash'lanır ve istek geri yönlendirilir. Formu render ederken bunları şablon struct'ına
çek:

```rust
#[get("/")]
async fn index(ctx: Ctx, db: Db, flash: Flash, csrf: Csrf) -> Result<View<PostsIndex>> {
    let errors = flash.errors().await;
    let page = PostsIndex {
        app_name: ctx.config().app.name.clone(),
        posts: Post::query().order_by_desc(Post::ID).fetch_all(&db).await?,
        csrf_field: csrf.field().await,
        old_title: flash.old("title").await.unwrap_or_default(),
        title_error: errors.first("title").map(str::to_owned),
    };
    flash.clear_old().await;
    Ok(View::new(page))
}
```

```html
<input name="title" value="{{ old_title }}">
{% if let Some(message) = title_error %}<p class="error">{{ message }}</p>{% endif %}
```

`Flash` ve `Csrf`'in tamamı için [CSRF & Oturumlar](../security/csrf)'a bak.

## Flash mesajları

`flash.take()`, kuyruğa alınmış mesajları döndürür ve temizler; bunları düzeninde render
edersin:

```rust
flash: Vec<rustali::session::Message>,   // şablon struct'ında
// flash: flash.take().await,            // onu kurarken
```

```html
{% for message in flash %}
  <div class="flash {{ message.level }}">{{ message.text }}</div>
{% endfor %}
```

Bir `Message`'ın bir `level`'ı (`success`, `info`, `warning`, `error`) ve `text`'i vardır.

## Tera (çalışma zamanı, `tera` özelliği)

Çalışma zamanı şablonları için `tera` özelliğini etkinleştir. Motoru `TeraProvider` ile kaydet,
sonra serileştirilebilir bir bağlamla ada göre render et:

```rust
async fn index(views: Inject<TeraView>) -> Result<Html<String>> {
    views.render("posts/index.html", &serde_json::json!({ "posts": posts }))
}
```

Tera varsayılan olarak `views/**/*` okur (`view` bölümü aracılığıyla yapılandırılabilir). Derleme
zamanı güvenliği istediğinde Askama'yı kullan; tasarımcıların yeniden derleme olmadan şablonları
düzenlemesi gerektiğinde Tera'ya başvur.

## SSR ve JSON birlikte

Bir controller, bir route'ta bir HTML `View`'ı ve başka bir route'ta `Json`'ı sunabilir — aynı
handler tarzı, farklı dönüş tipi. Hatalar çağırana uyacak şekilde render edilir (tarayıcılar
için HTML sayfası, API istemcileri için problem+JSON); böylece tek bir controller ikisini de
temizce sunar. [Yanıtlar & Hatalar](responses)'a bak.
