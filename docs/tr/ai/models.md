# Modeller

`candle` backend'iyle, Rustali gerçek bir modeli yerel olarak çalıştırır — CPU'da, sürecinde. Bu
sayfa bir modelin nasıl seçildiğini, indirildiğini ve yüklendiğini ve tamamen çevrimdışı nasıl
çalıştırılacağını kapsar.

## Yerel inference'ı açmak

Yerel modeller isteğe bağlıdır. `candle` özelliğini etkinleştir ve backend'i yapılandırmada seç:

```toml
# Cargo.toml
rustali-ai = { git = "https://github.com/zfrtemel/rustali-ai", features = ["candle"] }
```

```bash
# .env
APP__AI__BACKEND=candle
APP__AI__MODEL=sentence-transformers/all-MiniLM-L6-v2
APP__AI__MODEL_PATH=
```

`candle` ile ilk derleme yavaştır — Candle'ı ve bağımlılıklarını derler — ama yalnızca bir kez.
Özellik olmadan, `backend = "candle"` sessiz bir fallback değil, net bir başlangıç hatasıdır;
böylece yanlış yapılandırılmış bir deployment hızlı başarısız olur.

## Bir model nasıl yüklenir

Bir sentence transformer üç dosyadır: ağırlıklar (`model.safetensors`), tokenizer (`tokenizer.json`)
ve mimari (`config.json`). Rustali bunları iki yoldan biriyle çözer:

1. **Yerel dizin** — `model_path` o üç dosyayı içeren bir klasöre işaret ediyorsa, doğrudan
   kullanılır. Ağ yok.
2. **Hugging Face Hub** — aksi hâlde `model` tarafından adlandırılan repo Hub'dan indirilir ve diske
   cache'lenir. İlk çalıştırma onu getirir (MiniLM için ~90 MB); sonraki çalıştırmalar cache'i
   okur.

İkisi de ayarlandığında `model_path` kazanır. Model başlangıçta **bir kez** yüklenir ve sürecin
ömrü boyunca bellekte tutulur — inference onu yeniden kullanır, istek başına asla yeniden yüklenmez.

```bash
# .env — tamamen çevrimdışı: uygulamayla gönderdiğin dosyalara işaret et
APP__AI__BACKEND=candle
APP__AI__MODEL_PATH=models/all-MiniLM-L6-v2
```

Repo id'si `owner/name` olmalıdır (örn. `sentence-transformers/all-MiniLM-L6-v2`).

## Bir model seçmek

Varsayılan, **`all-MiniLM-L6-v2`**, embedding'ler için iyi bir başlangıç noktasıdır: küçük (384
boyut), CPU'da hızlı ve cümle benzerliğinde güçlü. `model`'i Hub'daki herhangi bir uyumlu BERT
sentence transformer repo'suna ayarlayarak farklı birini seç.

Seçerken, şunları tart:

- **Boyut / hız** — daha büyük modeller daha doğrudur ama daha yavaş ve daha ağırdır. CPU'da, çoğu
  uygulama için küçük kazanır.
- **Boyutlar (dimensions)** — daha fazla boyut daha fazla nüans yakalayabilir ama daha fazla
  depolamaya ve hesaplamaya mal olur. Bir store'daki tüm vektörler **aynı** modelden gelmelidir.
- **Göreve uygunluk** — arama/RAG için ham bir dil modeli değil, *cümle embedding'leri* için
  eğitilmiş bir model (`sentence-transformers` ailesi) kullan.

Modeli değiştirirsen, **her şeyi yeniden embed et** — farklı modellerden gelen vektörler
karşılaştırılamaz.

## Çevrimdışı / üretimde çalıştırmak

- **Modeli uygulamayla gönder** ve `model_path`'i ayarla; böylece üretim Hub erişimine ihtiyaç
  duymaz ve başlangıç bir indirmeyi beklemez.
- **Ya da cache'i ön-ısıt** — build/deploy adımında ağ ile bir kez çalıştır, sonra disk içi
  cache'e güven.
- Yükleme provider başlangıcı sırasında gerçekleşir; bir indirme yoluyla, o ilk açılış ağda
  bloke olur. `model_path` onu tamamen önler.

## Performans notları

- Inference, CPU tensör matematiğidir ve blocking bir thread pool'da çalışır; böylece async
  web sunucusunu durdurmaz — ama yine de CPU kullanır. Ağır batch iş için, birçok tek çağrı yerine
  [`embed_batch`](embeddings#embedding-uretmek)'i tercih et ve büyük işleri bir
  [queue'ya alınmış worker](../deeper/queues)'dan yapmayı düşün.
- Model istekler arasında paylaşılır (bir yükleme, birçok kullanım); böylece bellek istek başına
  değil, bir kez ödenir.

## Model olmadan geliştirmek

Bir AI özelliği kurmak için `candle`'a ihtiyacın yok. Varsayılan `mock` backend'i, deterministik
çıktıyla ve modelsiz aynı API'yi uygular — handler'larını, route'larını, `VectorStore` bağlantını ve
testlerini ona karşı yaz, sonra gerçek inference için hazır olduğunda `backend`'i `candle`'a çevir.
Kodun değişmez.
