# Embedding'ler

Bir embedding, bir metin parçasını anlamını yakalayan bir sayı listesine — bir vektöre — çevirir.
Benzer şeyler ifade eden iki metin, benzer yönlere işaret eden vektörler alır; böylece "bunlar ne
kadar ilişkili?" sorusunu anahtar kelime eşleştirme yerine aritmetikle ölçebilirsin. Bu, semantik
aramanın ve [retrieval'ın (RAG)](rag) temelidir.

## Embedding üretmek

Handle'a `ctx.ai()` ile eriş, sonra tek bir string'i `embed` et ya da birçoğunu `embed_batch` et:

```rust
let ai = ctx.ai()?;

let v = ai.embed("the quick brown fox").await?;   // Vec<f32>
println!("{} dimensions", v.len());               // örn. all-MiniLM-L6-v2 için 384

let texts = vec!["first".to_owned(), "second".to_owned()];
let vectors = ai.embed_batch(&texts).await?;       // Vec<Vec<f32>>, girdi başına bir, sırayla
```

**Bir `embed` döngüsü yerine `embed_batch`'i tercih et.** Bir backend, metin başına bir yerine tüm
batch için bir model geçişi (ya da bir ağ gidiş-dönüşü) çalıştırır — birçok öğe için çok daha hızlı.

`ai.dimensions()`, önceden ihtiyacın varsa (örneğin bir veritabanı kolonunu boyutlandırmak için)
mevcut modelin ürettiği vektör uzunluğunu sana söyler.

## Embedding'leri karşılaştırmak

`cosine_similarity`, iki vektörü puanlar: `1.0` aynı yön (çok benzer), `0.0` ilişkisiz, `-1.0` zıt
anlamına gelir.

```rust
use rustali_ai::cosine_similarity;

let a = ai.embed("how do I reset my password?").await?;
let b = ai.embed("I forgot my login credentials").await?;
let c = ai.embed("what time does the store close?").await?;

assert!(cosine_similarity(&a, &b) > cosine_similarity(&a, &c));
```

Kötü girdide güvenlidir — farklı uzunluklar, boş ya da tümü sıfır vektörler, panik yapmak yerine
`0.0` döndürür.

## En benzeri bulmak

Gündelik kullanım, "bir query verildiğinde, metinlerimden hangileri en yakın?"tir. Query'yi embed et,
adayları embed et, benzerliğe göre sırala:

```rust
let query = ai.embed(user_question).await?;
let candidate_vecs = ai.embed_batch(&candidates).await?;

let mut ranked: Vec<(&String, f32)> = candidates
    .iter()
    .zip(&candidate_vecs)
    .map(|(text, vec)| (text, cosine_similarity(&query, vec)))
    .collect();
ranked.sort_by(|a, b| b.1.total_cmp(&a.1));   // en iyi önce

for (text, score) in ranked.iter().take(3) {
    println!("{score:.2}  {text}");
}
```

Bunu etrafta tuttuğun bir koleksiyon üzerinde yaptığında, aday'ları her seferinde yeniden embed etme
— vektörlerini bir kez sakla. [`VectorStore`](rag)'un tam olarak senin için yaptığı budur.

## Vektörleri saklamak

Bir embedding yalnızca `Vec<f32>`'tir; bu yüzden onu istediğin gibi kalıcı kılabilirsin — bir JSON
kolonu, bir blob ya da özel bir vektör veritabanı. İki ipucu:

- **Onu hangi modelin ürettiğini kaydet.** Farklı modellerden gelen vektörler karşılaştırılamaz;
  model değiştirirsen yeniden embed etmelisin.
- **Boyut model başına sabittir** (`ai.dimensions()`); bu yüzden sabit genişlikli bir kolon çalışır.

Arama yerleşik, süreç içi bir depo için [`VectorStore`](rag)'u kullan.

## Hangi model kullanılır

`candle` backend'iyle, embedding'ler CPU'da çalışan yerel bir sentence transformer'dan gelir
(varsayılan `all-MiniLM-L6-v2`, 384 boyut). `mock` ile, deterministik yer tutuculardır — aynı metin
her zaman aynı vektörü verir; böylece testler kararlıdır, ama sayılar gerçek bir anlam taşımaz. Bir
modeli seçmek ve yüklemek için [Modeller](models)'e bak.
