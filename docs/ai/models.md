# Models

With the `candle` backend, Rustali runs a real model locally — on the CPU, in your process.
This page covers how a model is chosen, downloaded and loaded, and how to run fully offline.

## Turning on local inference

Local models are opt-in. Enable the `candle` feature and select the backend in config:

```toml
# Cargo.toml
rustali-ai = { git = "https://github.com/zfrtemel/rustali-ai", features = ["candle"] }
```

```toml
# config/ai.toml
backend = "candle"
model = "sentence-transformers/all-MiniLM-L6-v2"
model_path = ""
```

The first build with `candle` is slow — it compiles Candle and its dependencies — but only
once. Without the feature, `backend = "candle"` is a clear startup error, not a silent
fallback, so a misconfigured deploy fails fast.

## How a model is loaded

A sentence-transformer is three files: the weights (`model.safetensors`), the tokenizer
(`tokenizer.json`) and the architecture (`config.json`). Rustali resolves them in one of two
ways:

1. **Local directory** — if `model_path` points at a folder containing those three files, it is
   used directly. No network.
2. **Hugging Face Hub** — otherwise the repo named by `model` is downloaded from the Hub and
   cached on disk. The first run fetches it (~90 MB for MiniLM); later runs read the cache.

`model_path` wins when both are set. The model loads **once**, at startup, and is kept in
memory for the life of the process — inference reuses it, it is never reloaded per request.

```toml
# fully offline: point at files you shipped with the app
backend = "candle"
model_path = "models/all-MiniLM-L6-v2"
```

The repo id must be `owner/name` (e.g. `sentence-transformers/all-MiniLM-L6-v2`).

## Choosing a model

The default, **`all-MiniLM-L6-v2`**, is a good starting point for embeddings: small (384
dimensions), fast on a CPU, and strong at sentence similarity. Pick a different one by setting
`model` to any compatible BERT sentence-transformer repo on the Hub.

When choosing, weigh:

- **Size / speed** — bigger models are more accurate but slower and heavier. On a CPU, small
  wins for most apps.
- **Dimensions** — more dimensions can capture more nuance but cost more storage and compute.
  All vectors in one store must come from the **same** model.
- **Task fit** — use a model trained for *sentence embeddings* (the `sentence-transformers`
  family) for search/RAG, not a raw language model.

If you change the model, **re-embed everything** — vectors from different models are not
comparable.

## Running offline / in production

- **Ship the model with the app** and set `model_path`, so production needs no Hub access and
  startup doesn't wait on a download.
- **Or pre-warm the cache** in your build/deploy step by running once with network, then rely on
  the on-disk cache.
- Loading happens during provider startup; with a download path, that first boot blocks on the
  network. `model_path` avoids it entirely.

## Performance notes

- Inference is CPU tensor math and runs on a blocking thread pool, so it doesn't stall the async
  web server — but it still uses CPU. For heavy batch work, prefer
  [`embed_batch`](embeddings#producing-embeddings) over many single calls, and consider doing
  large jobs from a [queued worker](../deeper/queues).
- The model is shared across requests (one load, many uses), so memory is paid once, not per
  request.

## Developing without a model

You don't need `candle` to build an AI feature. The default `mock` backend implements the same
API with deterministic output and no model — write your handlers, routes, `VectorStore` wiring
and tests against it, then flip `backend` to `candle` when you're ready for real inference.
Your code does not change.
