# View'lar

View'lar HTML render eder. Rustali varsayılan olarak **Askama** kullanır; Askama template'leri
Rust'a derler ve derleme zamanında denetler — eksik bir değişken ya da template'teki bir yazım
hatası, üretimde bir 500 olarak değil, derleme sırasında bir hata olarak karşına çıkar. Hot reload
ile çalışma zamanı template'leri istersen, **Tera** backend'i bir özelliğin arkasında hazır
bekliyor.

## Askama (varsayılan)

Bir template, `Template`'i türeten ve bir template dosyasını adres gösteren bir struct'tır.
Struct'ın alanları, template içinde kullanabileceğin değişkenlerdir.

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

Template'ler varsayılan olarak `templates/` altında bulunur. Askama, crate kökündeki bir
`askama.toml` dosyasını okur:

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

`View::new(page)`, bir `200 text/html` response'u döndürür. `with_status` ile farklı bir durum
kodu ayarlayabilirsin — geçersiz bir formu yeniden render ederken işe yarar:

```rust
Ok(View::new(login_page).with_status(StatusCode::UNPROCESSABLE_ENTITY))
```

`View<T>` de sonuçta bir `IntoResponse` olduğu için, `Result<View<T>>` döndüren bir handler da
diğerleri gibi otomatik hata render'ından yararlanır.

### Bir template struct'ı bir view modelidir

Template struct'ı, sayfanın ihtiyaç duyduğu verinin tam olarak kendisidir — fazlası değil.
Entity'leri olduğu gibi aktarmak yerine veriyi view'a göre şekillendir: biçimlenmiş bir tarihi,
bir görünen adı ya da bir bayrağı önceden hesapla. [Sayfalanmış](../database/queries#pagination)
bir `Page<T>`'nin bir `map`'i olduğundan, sayfalama sayılarını koruyarak satırları bir view
modeline dönüştürebilirsin:

```rust
let page = Post::query().paginate(&db, 1, 20).await?
    .map(|post| PostRow { title: post.title, excerpt: truncate(&post.body, 140) });
```

Sunum mantığını template'te değil handler'da (düz Rust) tutmak, hem template'i sade tutar hem de
mantığı test edilebilir kılar.

## Düzenler (layout)

Askama template kalıtımını destekler. Bir temel düzen bloklar tanımlar; sayfalar bu düzeni
genişletip bloklarını doldurur.

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

Yeniden kullanılabilir bir parçayı başka bir template'in içine `{% include %}` ile alırsın. Parça,
kendisini dahil eden template'le aynı bağlamı görür:

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

## Bir bakışta template söz dizimi

Askama'nın söz dizimi Jinja'ya benzer. En sık kullanacağın parçalar şunlar:

| Söz dizimi | Yaptığı |
|---|---|
| <code v-pre>{{ expr }}</code> | Bir değer yazdırır (`.html`'de otomatik kaçışlı). |
| `{% if cond %} … {% else %} … {% endif %}` | Koşul. |
| `{% if let Some(x) = opt %} … {% endif %}` | Bir `Option`'ı/deseni eşler (gerçek Rust desenleri). |
| `{% for item in items %} … {% endfor %}` | Döngü; `loop.index` 1'den başlayan konumu verir. |
| `{% extends "base.html" %}` / `{% block %}` | Düzen kalıtımı. |
| `{% include "partial.html" %}` | Başka bir template'i satır içine alır. |
| <code v-pre>{{ value\|filter }}</code> | Bir filtre uygular, örn. `\|safe`, `\|upper`. |

Template'ler Rust'a derlendiği için içlerindeki ifadeler de tip denetiminden geçer: var olmayan bir
metodu çağırmak ya da struct'ta bulunmayan bir alanı yazdırmak `cargo build`'i başarısız kılar.

## Kaçış (escaping)

Askama, `.html` template'lerinde HTML'i otomatik olarak kaçışlar. Güvendiğin, önceden render
edilmiş bir dizeyi (örneğin bir CSRF gizli girdisini) olduğu gibi basmak için onu `safe` filtresinden
geçir:

```html
<form method="post" action="/posts">
  {{ csrf_field|safe }}
  ...
</form>
```

## Formlar: hatalar ve eski girdi

[Validation'dan geçen](validation) bir form gönderimi başarısız olduğunda, hatalar ve gönderilen
değerler session'a flash'lanır ve request geri yönlendirilir. Formu render ederken bunları template
struct'ına çekersin:

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

`Flash` ve `Csrf`'in tüm ayrıntıları için [CSRF & Session'lar](../security/csrf)'a bak.

## Flash mesajları

`flash.take()`, queue'daki mesajları döndürür ve queue'yu temizler; bunları düzeninde render
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

Bir `Message`'ın bir `level`'ı (`success`, `info`, `warning`, `error`) ve bir `text`'i vardır.

## Tera (çalışma zamanı, `tera` özelliği)

Çalışma zamanı template'leri için `tera` özelliğini etkinleştir. Motoru `TeraProvider` ile kaydet,
ardından serileştirilebilir bir bağlam vererek template'i adına göre render et:

```rust
async fn index(views: Inject<TeraView>) -> Result<Html<String>> {
    views.render("posts/index.html", &serde_json::json!({ "posts": posts }))
}
```

Tera varsayılan olarak `views/**/*` okur (bu, `view` bölümünden yapılandırılabilir). Derleme zamanı
güvenliği istediğinde Askama'yı kullan; tasarımcıların yeniden derleme yapmadan template'leri
düzenleyebilmesi gerektiğinde Tera'ya geç.

## SSR ve JSON birlikte

Bir controller, bir route'ta HTML bir `View`, başka bir route'ta `Json` sunabilir — aynı handler
tarzı, farklı dönüş tipi. Hatalar çağıran tarafa göre render edilir (tarayıcılar için HTML sayfası,
API istemcileri için problem+JSON); böylece tek bir controller ikisini de derli toplu biçimde
sunar. [Response'lar & Hatalar](responses)'a bak.
