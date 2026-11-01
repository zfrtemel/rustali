# AI

Rustali apps can do AI **in the same Rust process** as the web app — no separate Python
service to run, deploy and keep in sync. This lives in a separate package,
[`rustali-ai`](https://github.com/zfrtemel/rustali-ai), because the local-model dependency is
heavy and not every app needs it; you add it only when you do.

Two capabilities, behind one handle you reach with `ctx.ai()`:

- **Embeddings** — turn text into vectors you can compare. The basis of semantic search and
  retrieval. See [Embeddings](embeddings).
- **Chat** — text generation: a prompt (or a conversation) in, a reply out. See [Chat](chat).

Put them together and you have **retrieval-augmented generation (RAG)**: embeddings find the
relevant text, chat writes the answer. See [Retrieval (RAG)](rag).

## Why a separate package

The default build stays lean and fast. Local inference pulls in
[Candle](https://github.com/huggingface/candle) (Hugging Face's Rust ML library) and a large
dependency tree; keeping it out of the main framework means apps that don't use AI never pay
for it. Apps that do just add one dependency.

## Install

```toml
[dependencies]
rustali-ai = { git = "https://github.com/zfrtemel/rustali-ai" }

# for local model inference (embeddings on the CPU, in-process):
rustali-ai = { git = "https://github.com/zfrtemel/rustali-ai", features = ["candle"] }
```

`candle` is **opt-in** — without it you still get the full API, backed by a deterministic
`mock` backend that needs no model and no network (perfect for tests and for building the
wiring before you pick a model).

## Set it up

Add the provider when you build the app, then reach the handle from any handler:

```rust
use rustali_ai::{AiProvider, AiContextExt};

// App::builder().provider(AiProvider).controllers().run().await

#[get("/similar")]
async fn similar(ctx: Ctx) -> Result<String> {
    let ai = ctx.ai()?;                    // the registered AI handle
    let a = ai.embed("the cat sat on the mat").await?;
    let b = ai.embed("a feline rested on the rug").await?;
    Ok(format!("{:.2}", rustali_ai::cosine_similarity(&a, &b)))
}
```

Configure it in `config/ai.toml`:

```toml
backend = "mock"                                    # mock | candle
model = "sentence-transformers/all-MiniLM-L6-v2"    # HF repo id (candle backend)
model_path = ""                                     # local dir; wins over `model` when set
```

## The backends

| Backend | Feature | What it is |
|---|---|---|
| `mock` | — (default) | Deterministic fake output. No model, no network. For tests and local dev. |
| `candle` | `candle` | Local inference with Candle: a sentence-transformer on the CPU, in your process. |

Both backends sit behind the same traits, so you develop against `mock` and switch to
`candle` with a config change — your code does not move. The traits are provider-agnostic, so
remote providers (OpenAI, Anthropic) can slot in behind them later.

## Where to next

- [Embeddings](embeddings) — text to vectors, and comparing them.
- [Chat](chat) — text generation.
- [Retrieval (RAG)](rag) — the `VectorStore`, and the retrieve-then-generate loop.
- [Models](models) — how a model is loaded, which one to pick, and running it offline.
