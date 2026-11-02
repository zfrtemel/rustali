# Sohbet

Sohbet metin üretimidir: modele bir istem — ya da bütün bir konuşma — verirsin ve bir yanıt yazar.
Bir RAG uygulamasının diğer yarısıdır (getirim gerçekleri bulur, sohbet yanıtı yazar) ve özetler,
taslaklar ve asistanlar için tek başına da yararlıdır.

## Tek seferlik bir istem

`prompt`, tek bir soru için kısayoldur:

```rust
let ai = ctx.ai()?;
let reply = ai.prompt("Write a one-line summary of the borrow checker.").await?;
```

## Bir konuşma

Çok turlu bağlam için — bir sistem talimatı, önceki turlar — mesajları kur ve `chat`'i çağır:

```rust
use rustali_ai::ChatMessage;

let reply = ai.chat(&[
    ChatMessage::system("You are a terse assistant. Answer in one sentence."),
    ChatMessage::user("What is ownership in Rust?"),
    ChatMessage::assistant("Each value has a single owner; it's dropped when the owner goes out of scope."),
    ChatMessage::user("And borrowing?"),
]).await?;
```

Üç rol, sohbet modellerinin nasıl istemlendiğini yansıtır:

| Rol | Kurulduğu | Amaç |
|---|---|---|
| system | `ChatMessage::system(..)` | Asistanı yönlendiren talimatlar. |
| user | `ChatMessage::user(..)` | Kişinin söylediği. |
| assistant | `ChatMessage::assistant(..)` | Çok turlu bir sohbette bağlam için önceki bir yanıt. |

`prompt(text)`, yalnızca `chat(&[ChatMessage::user(text)])`'tir.

## Sohbet kullanılabilir mi?

Sohbet ve embedding'ler **bağımsızdır** — bir tutamaç birini, diğerini ya da ikisini de sağlayabilir.
Kullanmadan önce kontrol et ve yapılandırılmış bir sohbet arka ucu yoksa bir hata bekle:

```rust
if ai.has_chat() {
    let reply = ai.prompt("hello").await?;
}
```

Varsayılan `mock` arka ucuyla, `prompt` / `chat`, son kullanıcı mesajını yankılayan **deterministik
bir taslak** döndürür. Bu, gerçek bir model yerindeyken önce istek akışını, route'ları ve şablonları
kurup test etmek için yeterlidir — taslağı gerçek bir arka uçla değiştirdiğinde kodunun şekli
değişmez.

## Arka uçlar

Metin üretimi `Chat` trait'inin arkasındadır; böylece herhangi bir arka uç — yerel bir model ya da
`OpenAI` ya da Anthropic gibi uzak bir provider — aynı `generate(messages) -> String`'i uygular ve
aynı `ai.chat(..)` / `ai.prompt(..)` çağrılarına takılır. Bugün gelen arka uç deterministik
`mock`'tur; gerçek üretim arka uçları, çağrı yerlerini değiştirmeden bu trait'in arkasına takılır.

::: tip Yavaş çağrıları istek yolundan uzak tut
Üretim saniyeler sürebilir. Akışlanmayan, kullanıcıya dönük her şey için, onu bir
[kuyruğa alınmış işten](../deeper/queues) çalıştırmayı ve sonucu, yavaş bir üçüncü taraf API
çağrısında yapacağın gibi, bant dışı teslim etmeyi düşün.
:::
