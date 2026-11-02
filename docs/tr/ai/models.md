# Modeller

`candle` arka ucuyla, Rustali gerçek bir modeli yerel olarak çalıştırır — CPU'da, sürecinde. Bu
sayfa bir modelin nasıl seçildiğini, indirildiğini ve yüklendiğini ve tamamen çevrimdışı nasıl
çalıştırılacağını kapsar.

## Yerel çıkarımı açmak

Yerel modeller isteğe bağlıdır. `candle` özelliğini etkinleştir ve arka ucu yapılandırmada seç:

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
Özellik olmadan, `backend = "candle"` sessiz bir geri dönüş değil, net bir başlangıç hatasıdır;
böylece yanlış yapılandırılmış bir dağıtım hızlı başarısız olur.

## Bir model nasıl yüklenir

Bir cümle dönüştürücü üç dosyadır: ağırlıklar (`model.safetensors`), tokenizer (`tokenizer.json`) ve
mimari (`config.json`). Rustali bunları iki yoldan biriyle çözer:

1. **Yerel dizin** — `model_path` o üç dosyayı içeren bir klasöre işaret ediyorsa, doğrudan
   kullanılır. Ağ yok.
2. **Hugging Face Hub** — aksi hâlde `model` tarafından adlandırılan repo Hub'dan indirilir ve diske
   önbelleğe alınır. İlk çalıştırma onu getirir (MiniLM için ~90 MB); sonraki çalıştırmalar önbelleği
   okur.

İkisi de ayarlandığında `model_path` kazanır. Model başlangıçta **bir kez** yüklenir ve sürecin
ömrü boyunca bellekte tutulur — çıkarım onu yeniden kullanır, istek başına asla yeniden yüklenmez.

```bash
# .env — tamamen çevrimdışı: uygulamayla gönderdiğin dosyalara işaret et
APP__AI__BACKEND=candle
APP__AI__MODEL_PATH=models/all-MiniLM-L6-v2
```

Repo id'si `owner/name` olmalıdır (örn. `sentence-transformers/all-MiniLM-L6-v2`).

## Bir model seçmek

Varsayılan, **`all-MiniLM-L6-v2`**, embedding'ler için iyi bir başlangıç noktasıdır: küçük (384
boyut), CPU'da hızlı ve cümle benzerliğinde güçlü. `model`'i Hub'daki herhangi bir uyumlu BERT cümle
dönüştürücü repo'suna ayarlayarak farklı birini seç.

Seçerken, şunları tart:

- **Boyut / hız** — daha büyük modeller daha doğrudur ama daha yavaş ve daha ağırdır. CPU'da, çoğu
  uygulama için küçük kazanır.
- **Boyutlar** — daha fazla boyut daha fazla nüans yakalayabilir ama daha fazla depolamaya ve
  hesaplamaya mal olur. Bir depodaki tüm vektörler **aynı** modelden gelmelidir.
- **Göreve uygunluk** — arama/RAG için ham bir dil modeli değil, *cümle embedding'leri* için
  eğitilmiş bir model (`sentence-transformers` ailesi) kullan.

Modeli değiştirirsen, **her şeyi yeniden embed et** — farklı modellerden gelen vektörler
karşılaştırılamaz.

## Çevrimdışı / üretimde çalıştırmak

- **Modeli uygulamayla gönder** ve `model_path`'i ayarla; böylece üretim Hub erişimine ihtiyaç
  duymaz ve başlangıç bir indirmeyi beklemez.
- **Ya da önbelleği ön-ısıt** — derleme/dağıtım adımında ağ ile bir kez çalıştır, sonra disk içi
  önbelleğe güven.
- Yükleme provider başlangıcı sırasında gerçekleşir; bir indirme yoluyla, o ilk açılış ağda
  bloke olur. `model_path` onu tamamen önler.

## Performans notları

- Çıkarım CPU tensör matematiğidir ve bloke eden bir iş parçacığı havuzunda çalışır; böylece async
  web sunucusunu durdurmaz — ama yine de CPU kullanır. Ağır grup işi için, birçok tek çağrı yerine
  [`embed_batch`](embeddings#embedding-üretmek)'i tercih et ve büyük işleri bir
  [kuyruğa alınmış worker](../deeper/queues)'dan yapmayı düşün.
- Model istekler arasında paylaşılır (bir yükleme, birçok kullanım); böylece bellek istek başına
  değil, bir kez ödenir.

## Model olmadan geliştirmek

Bir AI özelliği kurmak için `candle`'a ihtiyacın yok. Varsayılan `mock` arka ucu, deterministik
çıktıyla ve modelsiz aynı API'yi uygular — handler'larını, route'larını, `VectorStore` bağlantını ve
testlerini ona karşı yaz, sonra gerçek çıkarım için hazır olduğunda `backend`'i `candle`'a çevir.
Kodun değişmez.
