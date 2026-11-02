# Getirim (RAG)

**Getirimle artırılmış üretim**, kendi içeriğini kullanarak bir soruyu yanıtlar: önce soruyla ilgili
pasajları *getir*, sonra bir sohbet modelinden onlara dayalı bir yanıt *üret*mesini iste. Bir modeli
eğitmeden belgelerini, politikalarını, verilerini bilen bir asistanı bu şekilde yaparsın.

Rustali sana her iki yarıyı da verir:

- [Embedding'ler](embeddings) + bir [`VectorStore`](#vectorstore) ile **getir**.
- Getirilen pasajları bağlam olarak besleyerek [sohbet](chat) ile **üret**.

## VectorStore

`VectorStore`, metinleri ve embedding'lerini bir arada tutar ve onları anlama göre arar. Getirmeyi
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
| `add_with(ai, text, Some(payload))` | Aramada döndürülen üst veriyle (bir kaynak id/URL) sakla. |
| `add_batch(ai, &texts)` | Birçoğunu bir kerede embed et ve sakla (bir model geçişi). |
| `search(ai, query, top_k)` | `query`'yi embed et, en benzer `top_k`'yı döndür, en iyi önce. |
| `len()` / `is_empty()` / `clear()` | Boyut ve sıfırlama. |

Her isabet bir `SearchResult { text, payload, score }`'tur — `score` sorguya kosinüs
benzerliğidir, `payload` sakladığın şeydir (kaynağı atıfta bulunmak için kullanışlı).

### Kapsam

**Bellektedir** ve aramada her girişi tarar — birkaç taneden birkaç bin pasaja kadar ideal. Bunun
ötesinde, özel bir vektör veritabanına geç; `VectorStore` API'si orada çağıracağın şeyin referans
şeklidir.

Vektörler senin için kalıcı kılınmaz. Depoyu başlangıçta gerçek kaynağından (dosyalar, bir DB
tablosu) yeniden kur ya da metinleri kalıcı kıl ve açılışta yeniden embed et.

## Getir-sonra-üret döngüsü

İki yarıyı bir araya getir: pasajları getir, sonra sohbetten yalnızca onları kullanarak yanıt
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

- **Parça boyutu** — belirli olacak kadar küçük ama tek başına durabilecek kadar büyük pasajlar ekle
  (bir paragraf ya da bir bölüm, bütün bir belge değil).
- **`top_k`** — modele kaç pasaj besleneceği. Çok az bağlamı kaçırır; çok fazla onu seyreltir ve
  token'a mal olur. 3–5 civarında başla ve ayarla.

## Depoyu kurmak

Depoyu istek başına değil, bir kez (başlangıçta ya da içeriğin değiştiğinde) doldur — embedding
pahalı adımdır. Yaygın bir şekil, onu belgelerini okuyan, onları embed eden ve hazır `VectorStore`'u
handler'ların sorgulaması için tutan bir [provider](../deeper/providers)'dan bir servis olarak
kaydetmektir.

## Test

`mock` arka ucu, getirmeyi modelsiz test edilebilir kılar: aynı metin aynı vektöre embed olur;
böylece tam-metin bir sorgu `1.0` puan alır ve ilk sıralanır. Bağlantıyı — add, search, sıralama,
`top_k`, payload'lar — hiçbir şey indirmeden iddia edebilirsin. Gerçek anlamsal eşleştirme için
`candle` arka ucunu (ya da daha sonra uzak birini) takas et.
