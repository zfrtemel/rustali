# Your First App

```bash
cargo install rustali-cli     # once 0.1.0 is published
rustali new blog
cd blog
cargo run -- migrate          # create the SQLite schema
cargo run                     # http://127.0.0.1:8000
```

`rustali new` scaffolds a conventional project:

```text
blog/
├── Cargo.toml
├── .env
├── config/
│   ├── app.toml          # name, env, debug, key
│   └── database.toml     # connection URL (SQLite by default)
├── db/                   # SQLite file + migrations + schema snapshot
├── src/
│   ├── main.rs           # App::builder()...run()  (≈ 8 lines)
│   └── controllers.rs    # a home controller: SSR "/" and JSON "/api/ping"
└── templates/
    └── home.html
```

## Add a resource

```bash
rustali make:model Post          # src/models/post.rs  (#[derive(Entity)])
rustali make:controller Post     # src/controllers/post.rs
rustali make:migration create_posts
```

Edit the model's columns, then generate the migration from the change:

```bash
cargo run -- migrate:diff "create posts"   # writes a reviewed migration + snapshot
cargo run -- migrate
```

Mount the controller (`App::builder().controllers()` already picks up every
`#[controller]` in the binary) and you have CRUD. See
[Routing & Controllers](../basics/routing.md) next.
