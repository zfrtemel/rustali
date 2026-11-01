# Embeddings

An embedding turns a piece of text into a list of numbers — a vector — that captures its
meaning. Two texts that mean similar things get vectors that point in similar directions, so
you can measure "how related are these?" with arithmetic instead of keyword matching. This is
the foundation of semantic search and [retrieval (RAG)](rag).

## Producing embeddings

Reach the handle with `ctx.ai()`, then `embed` one string or `embed_batch` many:

```rust
let ai = ctx.ai()?;

let v = ai.embed("the quick brown fox").await?;   // Vec<f32>
println!("{} dimensions", v.len());               // e.g. 384 for all-MiniLM-L6-v2

let texts = vec!["first".to_owned(), "second".to_owned()];
let vectors = ai.embed_batch(&texts).await?;       // Vec<Vec<f32>>, one per input, in order
```

**Prefer `embed_batch` over a loop of `embed`.** A backend runs one model pass (or one
network round-trip) for the whole batch instead of one per text — much faster for many items.

`ai.dimensions()` tells you the vector length the current model produces, if you need it up
front (for example to size a database column).

## Comparing embeddings

`cosine_similarity` scores two vectors: `1.0` means the same direction (very similar), `0.0`
unrelated, `-1.0` opposite.

```rust
use rustali_ai::cosine_similarity;

let a = ai.embed("how do I reset my password?").await?;
let b = ai.embed("I forgot my login credentials").await?;
let c = ai.embed("what time does the store close?").await?;

assert!(cosine_similarity(&a, &b) > cosine_similarity(&a, &c));
```

It is safe on bad input — different lengths, empty, or all-zero vectors return `0.0` rather
than panicking.

## Finding the most similar

The everyday use is "given a query, which of my texts are closest?" Embed the query, embed the
candidates, rank by similarity:

```rust
let query = ai.embed(user_question).await?;
let candidate_vecs = ai.embed_batch(&candidates).await?;

let mut ranked: Vec<(&String, f32)> = candidates
    .iter()
    .zip(&candidate_vecs)
    .map(|(text, vec)| (text, cosine_similarity(&query, vec)))
    .collect();
ranked.sort_by(|a, b| b.1.total_cmp(&a.1));   // best first

for (text, score) in ranked.iter().take(3) {
    println!("{score:.2}  {text}");
}
```

When you do this over a collection you keep around, don't re-embed the candidates every
time — store their vectors once. That is exactly what [`VectorStore`](rag) does for you.

## Storing vectors

An embedding is just `Vec<f32>`, so you can persist it however you like — a JSON column, a
blob, or a dedicated vector database. Two tips:

- **Record which model produced it.** Vectors from different models are not comparable; if you
  change models you must re-embed.
- **The dimension is fixed per model** (`ai.dimensions()`), so a fixed-width column works.

For an in-process store with search built in, use [`VectorStore`](rag).

## What model is used

With the `candle` backend, embeddings come from a local sentence-transformer (default
`all-MiniLM-L6-v2`, 384 dimensions) running on the CPU. With `mock`, they are deterministic
placeholders — identical text always yields an identical vector, so tests are stable, but the
numbers carry no real meaning. See [Models](models) for choosing and loading a model.
