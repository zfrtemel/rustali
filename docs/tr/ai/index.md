# Yapay Zekâ

Rustali uygulamaları, yapay zekâyı web uygulamasıyla **aynı Rust sürecinde** yapabilir —
çalıştırılacak, dağıtılacak ve senkronize tutulacak ayrı bir Python servisi yok. Bu, ayrı bir
pakette, [`rustali-ai`](https://github.com/zfrtemel/rustali-ai)'de yaşar, çünkü yerel-model
bağımlılığı ağırdır ve her uygulama ona ihtiyaç duymaz; onu yalnızca ihtiyaç duyduğunda eklersin.

`ctx.ai()` ile eriştiğin tek bir handle'ın arkasında iki yetenek:

- **Embedding'ler** — metni karşılaştırabileceğin vektörlere çevir. Semantik aramanın ve
  retrieval'ın temeli. [Embedding'ler](embeddings)'e bak.
- **Chat** — metin üretimi: bir prompt (ya da bir konuşma) girer, bir yanıt çıkar. [Chat](chat)'e
  bak.

İkisini bir araya getir ve **retrieval-augmented generation (RAG)** elde edersin: embedding'ler
ilgili metni bulur, chat yanıtı yazar. [Retrieval (RAG)](rag)'a bak.

## Neden ayrı bir paket

Varsayılan derleme hafif ve hızlı kalır. Yerel inference,
[Candle](https://github.com/huggingface/candle)'ı (Hugging Face'in Rust ML kütüphanesi) ve büyük bir
bağımlılık ağacını içeri çeker; onu ana framework'ün dışında tutmak, yapay zekâ kullanmayan
uygulamaların onun bedelini asla ödememesi anlamına gelir. Kullanan uygulamalar yalnızca bir
bağımlılık ekler.

## Kurulum

```toml
[dependencies]
rustali-ai = { git = "https://github.com/zfrtemel/rustali-ai" }

# yerel model inference için (CPU'da, süreç içi embedding'ler):
rustali-ai = { git = "https://github.com/zfrtemel/rustali-ai", features = ["candle"] }
```

`candle` **isteğe bağlıdır** — onsuz bile, hiçbir model ve ağ gerektirmeyen deterministik bir `mock`
backend'le desteklenen tam API'yi elde edersin (testler ve bir model seçmeden önce bağlantıyı kurmak
için mükemmel).

## Kur

Uygulamayı kurarken provider'ı ekle, sonra herhangi bir handler'dan handle'a eriş:

```rust
use rustali_ai::{AiProvider, AiContextExt};

// App::builder().provider(AiProvider).controllers().run().await

#[get("/similar")]
async fn similar(ctx: Ctx) -> Result<String> {
    let ai = ctx.ai()?;                    // kayıtlı AI handle'ı
    let a = ai.embed("the cat sat on the mat").await?;
    let b = ai.embed("a feline rested on the rug").await?;
    Ok(format!("{:.2}", rustali_ai::cosine_similarity(&a, &b)))
}
```

Onu `.env`'de yapılandır. `rustali-ai` ayrı bir paket olduğu için, ayarları yerleşik anahtarlar
yerine `APP__AI__*` kaçış yolunu (`ai` bölümü) kullanır:

```bash
# .env
APP__AI__BACKEND=mock                                     # mock | candle
APP__AI__MODEL=sentence-transformers/all-MiniLM-L6-v2     # HF repo id (candle arka ucu)
APP__AI__MODEL_PATH=                                      # yerel dizin; ayarlandığında model'i yener
```

## Backend'ler

| Backend | Özellik | Olduğu şey |
|---|---|---|
| `mock` | — (varsayılan) | Deterministik sahte çıktı. Model yok, ağ yok. Testler ve yerel geliştirme için. |
| `candle` | `candle` | Candle ile yerel inference: CPU'da, sürecinde bir sentence transformer. |

Her iki backend de aynı trait'lerin arkasında oturur; böylece `mock`'a karşı geliştirir ve bir
yapılandırma değişikliğiyle `candle`'a geçersin — kodun taşınmaz. Trait'ler provider'dan bağımsızdır;
böylece uzak provider'lar (`OpenAI`, Anthropic) daha sonra onların arkasına takılabilir.

## Sıradaki nereye

- [Embedding'ler](embeddings) — metinden vektörlere ve onları karşılaştırma.
- [Chat](chat) — metin üretimi.
- [Retrieval (RAG)](rag) — `VectorStore` ve getir-sonra-üret döngüsü.
- [Modeller](models) — bir modelin nasıl yüklendiği, hangisinin seçileceği ve onu çevrimdışı
  çalıştırma.
