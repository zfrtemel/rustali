# Chat

Chat, metin üretimidir: modele bir prompt — ya da bütün bir konuşma — verirsin ve bir yanıt yazar.
Bir RAG uygulamasının diğer yarısıdır (retrieval gerçekleri bulur, chat yanıtı yazar) ve özetler,
taslaklar ve asistanlar için tek başına da yararlıdır.

## Tek seferlik bir prompt

`prompt`, tek bir soru için kısayoldur:

```rust
let ai = ctx.ai()?;
let reply = ai.prompt("Write a one-line summary of the borrow checker.").await?;
```

## Bir konuşma

Çok turlu bağlam için — bir system talimatı, önceki turlar — mesajları kur ve `chat`'i çağır:

```rust
use rustali_ai::ChatMessage;

let reply = ai.chat(&[
    ChatMessage::system("You are a terse assistant. Answer in one sentence."),
    ChatMessage::user("What is ownership in Rust?"),
    ChatMessage::assistant("Each value has a single owner; it's dropped when the owner goes out of scope."),
    ChatMessage::user("And borrowing?"),
]).await?;
```

Üç rol, chat modellerine prompt'un nasıl verildiğini yansıtır:

| Rol | Kurulduğu | Amaç |
|---|---|---|
| system | `ChatMessage::system(..)` | Asistanı yönlendiren talimatlar. |
| user | `ChatMessage::user(..)` | Kişinin söylediği. |
| assistant | `ChatMessage::assistant(..)` | Çok turlu bir konuşmada bağlam için önceki bir yanıt. |

`prompt(text)`, yalnızca `chat(&[ChatMessage::user(text)])`'tir.

## Chat kullanılabilir mi?

Chat ve embedding'ler **bağımsızdır** — bir handle birini, diğerini ya da ikisini de sağlayabilir.
Kullanmadan önce kontrol et ve yapılandırılmış bir chat backend'i yoksa bir hata bekle:

```rust
if ai.has_chat() {
    let reply = ai.prompt("hello").await?;
}
```

Varsayılan `mock` backend'iyle, `prompt` / `chat`, son kullanıcı mesajını yankılayan **deterministik
bir taslak** döndürür. Bu, gerçek bir model yerindeyken önce istek akışını, route'ları ve şablonları
kurup test etmek için yeterlidir — taslağı gerçek bir backend'le değiştirdiğinde kodunun şekli
değişmez.

## Backend'ler

Metin üretimi `Chat` trait'inin arkasındadır; böylece herhangi bir backend — yerel bir model ya da
`OpenAI` ya da Anthropic gibi uzak bir provider — aynı `generate(messages) -> String`'i uygular ve
aynı `ai.chat(..)` / `ai.prompt(..)` çağrılarına takılır. Bugün gelen backend deterministik
`mock`'tur; gerçek üretim backend'leri, çağrı yerlerini değiştirmeden bu trait'in arkasına takılır.

::: tip Yavaş çağrıları istek yolundan uzak tut
Üretim saniyeler sürebilir. Stream edilmeyen, kullanıcıya dönük her şey için, onu bir
[queue'ya alınmış job'dan](../deeper/queues) çalıştırmayı ve sonucu, yavaş bir üçüncü taraf API
çağrısında yapacağın gibi, bant dışı teslim etmeyi düşün.
:::
