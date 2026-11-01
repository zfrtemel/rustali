---
layout: home

hero:
  name: Rustali
  text: A batteries-included web framework for Rust
  tagline: Everything you need to ship a web app — routing, ORM, migrations, views, auth and more — with Rust's safety and speed.
  actions:
    - theme: brand
      text: Get Started
      link: /getting-started/first-app
    - theme: alt
      text: From Laravel
      link: /migrating/from-laravel
    - theme: alt
      text: View on GitHub
      link: https://github.com/zfrtemel/rustali

features:
  - title: Attribute routing
    details: Routes live on the handler; `cargo run -- routes` lists them all. No separate route file.
  - title: POCO entities
    details: One plain struct with #[derive(Entity)]. The SeaORM plumbing is generated and hidden.
  - title: Code-first migrations
    details: Change the struct, run migrate:diff, review the generated migration.
  - title: Layered filter pipeline
    details: Global → controller → action middleware, #[authorize], exception filters — all tower layers.
  - title: Type-safe views
    details: Askama templates checked at compile time by default; Tera available for runtime flexibility.
  - title: SSR + API, one controller
    details: Return HTML or JSON per route. Errors are negotiated to match the client automatically.
---
