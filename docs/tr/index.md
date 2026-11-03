---
layout: home

hero:
  name: Rustali
  text: Rust için her şeyi içinde barındıran web framework'ü
  tagline: Bir web uygulamasını yayına almak için gereken her şey — routing, ORM, migration, view'lar, authentication ve dahası — Rust'ın güvenliği ve hızıyla.
  actions:
    - theme: brand
      text: Başla
      link: /tr/getting-started/first-app
    - theme: alt
      text: GitHub'da Görüntüle
      link: https://github.com/zfrtemel/rustali

features:
  - title: Attribute routing
    details: Route'lar handler'ın üzerinde yaşar; `cargo run -- routes` hepsini listeler. Ayrı bir route dosyası yok.
  - title: Düz struct entity'ler
    details: #[entity] ile tek bir düz struct. SeaORM boilerplate'i üretilir ve gizlenir.
  - title: Code-first migration
    details: Struct'ı değiştir, migrate:diff çalıştır, üretilen migration'ı gözden geçir. Kontrol hep sende.
  - title: Katmanlı filter pipeline
    details: Global → controller → action middleware, #[authorize], exception filter'ları — hepsi tower layer'ı.
  - title: Tip güvenli view'lar
    details: Askama template'leri varsayılan olarak derleme zamanında kontrol edilir; runtime esnekliği için Tera mevcut.
  - title: SSR + API, tek controller
    details: Her route'ta HTML ya da JSON döndür. Hatalar client'a uyacak şekilde otomatik negotiate edilir.
  - title: Süreç içi yapay zekâ
    details: rustali-ai ile aynı Rust süreci içinde embedding, chat ve RAG — Python servisi gerekmez.
---
