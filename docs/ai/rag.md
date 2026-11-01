# Retrieval (RAG)

**Retrieval-augmented generation** answers a question using your own content: first *retrieve*
the passages relevant to the question, then ask a chat model to *generate* an answer grounded
in them. It is how you make an assistant that knows your docs, your policies, your data —
without training a model.

Rustali gives you both halves:

- **Retrieve** with [embeddings](embeddings) + a [`VectorStore`](#the-vectorstore).
- **Generate** with [chat](chat), feeding the retrieved passages in as context.

## The VectorStore

`VectorStore` keeps texts and their embeddings together and searches them by meaning. It turns
retrieval into two calls — `add` and `search` — so you don't hand-roll the embed-and-rank loop.

```rust
use rustali_ai::VectorStore;

let ai = ctx.ai()?;
let mut store = VectorStore::new();

// Add your knowledge. Each text is embedded once, when added.
store.add(&ai, "Rust has no garbage collector.").await?;
store.add(&ai, "The borrow checker enforces ownership at compile time.").await?;
store.add_with(&ai, "Cargo builds, tests and publishes your crate.", Some("docs#cargo")).await?;

// Retrieve the most relevant entries for a question.
let hits = store.search(&ai, "how does Rust manage memory?", 3).await?;
for hit in &hits {
    println!("{:.2}  {}", hit.score, hit.text);   // best first
}
```

### Methods

| Method | Does |
|---|---|
| `add(ai, text)` | Embed `text` and store it. |
| `add_with(ai, text, Some(payload))` | Store with metadata (a source id/URL) returned on search. |
| `add_batch(ai, &texts)` | Embed and store many at once (one model pass). |
| `search(ai, query, top_k)` | Embed `query`, return the `top_k` most similar, best first. |
| `len()` / `is_empty()` / `clear()` | Size and reset. |

Each hit is a `SearchResult { text, payload, score }` — `score` is cosine similarity to the
query, `payload` is whatever you stored (handy for citing the source).

### Scope

It is **in memory** and scans every entry on search — ideal from a handful up to a few
thousand passages. Beyond that, move to a dedicated vector database; the `VectorStore` API is
the reference shape for what you'll call there.

Vectors are not persisted for you. Rebuild the store at startup from your source of truth
(files, a DB table), or persist the texts and re-embed on boot.

## The retrieve-then-generate loop

Put the two halves together: retrieve passages, then ask chat to answer using only them.

```rust
use rustali_ai::ChatMessage;

async fn answer(ctx: &AppContext, question: &str) -> Result<String> {
    let ai = ctx.ai()?;

    // 1. Retrieve the most relevant context (assume `store` was built at startup).
    let hits = store.search(&ai, question, 4).await?;
    let context = hits
        .iter()
        .map(|h| format!("- {}", h.text))
        .collect::<Vec<_>>()
        .join("\n");

    // 2. Generate an answer grounded in that context.
    let reply = ai.chat(&[
        ChatMessage::system(
            "Answer using only the context below. If it isn't there, say you don't know.",
        ),
        ChatMessage::user(format!("Context:\n{context}\n\nQuestion: {question}")),
    ]).await?;

    Ok(reply)
}
```

That is the whole pattern. The quality comes from two knobs you control:

- **Chunk size** — add passages that are small enough to be specific but large enough to stand
  alone (a paragraph or a section, not a whole document).
- **`top_k`** — how many passages to feed the model. Too few misses context; too many dilutes
  it and costs tokens. Start around 3–5 and tune.

## Building the store

Populate the store once (at startup, or when your content changes), not per request — embedding
is the expensive step. A common shape is to register it as a service from a
[provider](../deeper/providers) that reads your documents, embeds them, and keeps the ready
`VectorStore` for handlers to query.

## Testing

The `mock` backend makes retrieval testable with no model: identical text embeds to an
identical vector, so an exact-text query scores `1.0` and ranks first. You can assert the
wiring — add, search, ranking, `top_k`, payloads — without downloading anything. Swap in the
`candle` backend (or a remote one later) for real semantic matching.
