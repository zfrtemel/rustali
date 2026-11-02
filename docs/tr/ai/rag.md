# Retrieval (RAG)

**Retrieval-augmented generation**, kendi içeriğini kullanarak bir soruyu yanıtlar: önce soruyla
ilgili pasajları *getir* (retrieval), sonra bir chat modelinden onlara dayalı bir yanıt *üret*mesini
iste. Bir modeli eğitmeden belgelerini, politikalarını, verilerini bilen bir asistanı bu şekilde
yaparsın.

Rustali sana her iki yarıyı da verir:

- [Embedding'ler](embeddings) + bir [`VectorStore`](#vectorstore) ile **getir**.
- Getirilen pasajları bağlam olarak besleyerek [chat](chat) ile **üret**.

## VectorStore

`VectorStore`, metinleri ve embedding'lerini bir arada tutar ve onları anlama göre arar. Retrieval'ı
iki çağrıya çevirir — `add` ve `search` — böylece embed-ve-sırala döngüsünü elle yazmazsın.

```rust
use rustali_ai::VectorStore;

let ai = ctx.ai()?;
let mut store = VectorStore::new();

// Bilgini ekle. Her metin, eklendiğinde bir kez embed edilir.
store.add(&ai, "Rust has no garbage collector.").await?;
store.add(&ai, "The borrow checker enforces ownership at compile time.").await?;
store.add_with(&ai, "Cargo builds, tests and publishes your crate.", Some("docs#cargo")).await?;

// Bir soru için en ilgili girişleri getir.
let hits = store.search(&ai, "how does Rust manage memory?", 3).await?;
for hit in &hits {
    println!("{:.2}  {}", hit.score, hit.text);   // en iyi önce
}
```

### Metotlar

| Metot | Yaptığı |
|---|---|
| `add(ai, text)` | `text`'i embed et ve sakla. |
| `add_with(ai, text, Some(payload))` | Aramada döndürülen metadata ile (bir kaynak id/URL) sakla. |
| `add_batch(ai, &texts)` | Birçoğunu bir kerede embed et ve sakla (bir model geçişi). |
| `search(ai, query, top_k)` | `query`'yi embed et, en benzer `top_k`'yı döndür, en iyi önce. |
| `len()` / `is_empty()` / `clear()` | Boyut ve sıfırlama. |

Her hit bir `SearchResult { text, payload, score }`'tur — `score` sorguya cosine
benzerliğidir, `payload` sakladığın şeydir (kaynağı referans göstermek için kullanışlı).

### Kapsam

**Bellektedir** ve aramada her girişi tarar — birkaç taneden birkaç bin pasaja kadar ideal. Bunun
ötesinde, özel bir vektör veritabanına geç; `VectorStore` API'si orada çağıracağın şeyin referans
şeklidir.

Vektörler senin için persist edilmez. Store'u başlangıçta gerçek kaynağından (dosyalar, bir DB
tablosu) yeniden kur ya da metinleri persist et ve açılışta yeniden embed et.

## Getir-sonra-üret döngüsü

İki yarıyı bir araya getir: pasajları getir, sonra chat'ten yalnızca onları kullanarak yanıt
vermesini iste.

```rust
use rustali_ai::ChatMessage;

async fn answer(ctx: &AppContext, question: &str) -> Result<String> {
    let ai = ctx.ai()?;

    // 1. En ilgili bağlamı getir (`store`'un başlangıçta kurulduğunu varsay).
    let hits = store.search(&ai, question, 4).await?;
    let context = hits
        .iter()
        .map(|h| format!("- {}", h.text))
        .collect::<Vec<_>>()
        .join("\n");

    // 2. O bağlama dayalı bir yanıt üret.
    let reply = ai.chat(&[
        ChatMessage::system(
            "Answer using only the context below. If it isn't there, say you don't know.",
        ),
        ChatMessage::user(format!("Context:\n{context}\n\nQuestion: {question}")),
    ]).await?;

    Ok(reply)
}
```

Tüm desen bu kadar. Kalite, kontrol ettiğin iki düğmeden gelir:

- **Chunk boyutu** — spesifik olacak kadar küçük ama tek başına durabilecek kadar büyük pasajlar ekle
  (bir paragraf ya da bir bölüm, bütün bir belge değil).
- **`top_k`** — modele kaç pasaj besleneceği. Çok az bağlamı kaçırır; çok fazla onu seyreltir ve
  token'a mal olur. 3–5 civarında başla ve ayarla.

## Store'u kurmak

Store'u istek başına değil, bir kez (başlangıçta ya da içeriğin değiştiğinde) doldur — embedding
pahalı adımdır. Yaygın bir şekil, onu belgelerini okuyan, onları embed eden ve hazır `VectorStore`'u
handler'ların sorgulaması için tutan bir [provider](../deeper/providers)'dan bir servis olarak
kaydetmektir.

## Test

`mock` backend'i, retrieval'ı modelsiz test edilebilir kılar: aynı metin aynı vektöre embed olur;
böylece tam-metin bir query `1.0` skor alır ve ilk sıralanır. Bağlantıyı — add, search, sıralama,
`top_k`, payload'lar — hiçbir şey indirmeden assert edebilirsin. Gerçek semantik eşleştirme için
`candle` backend'ini (ya da daha sonra uzak birini) takas et.
