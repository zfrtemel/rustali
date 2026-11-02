---
layout: home

hero:
  name: Rustali
  text: Rust için her şeyi içinde barındıran web framework'ü
  tagline: Bir web uygulamasını yayına almak için gereken her şey — yönlendirme, ORM, migration, görünümler, kimlik doğrulama ve dahası — Rust'ın güvenliği ve hızıyla.
  actions:
    - theme: brand
      text: Başla
      link: /tr/getting-started/first-app
    - theme: alt
      text: GitHub'da Görüntüle
      link: https://github.com/zfrtemel/rustali

features:
  - title: Attribute yönlendirme
    details: Route'lar handler'ın üzerinde yaşar; `cargo run -- routes` hepsini listeler. Ayrı bir route dosyası yok.
  - title: Düz struct varlıklar
    details: #[derive(Entity)] ile tek bir düz struct. SeaORM altyapısı üretilir ve gizlenir.
  - title: Code-first migration
    details: Struct'ı değiştir, migrate:diff çalıştır, üretilen migration'ı gözden geçir.
  - title: Katmanlı filtre hattı
    details: Global → controller → action middleware, #[authorize], istisna filtreleri — hepsi tower katmanı.
  - title: Tip güvenli görünümler
    details: Askama şablonları varsayılan olarak derleme zamanında denetlenir; çalışma zamanı esnekliği için Tera mevcut.
  - title: SSR + API, tek controller
    details: Her route'ta HTML ya da JSON döndür. Hatalar istemciye uyacak şekilde otomatik pazarlık edilir.
  - title: Süreç içi yapay zekâ
    details: rustali-ai ile aynı Rust süreci içinde embedding, sohbet ve RAG — Python servisi gerekmez.
---
