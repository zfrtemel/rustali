# Chat

Chat is text generation: you give the model a prompt — or a whole conversation — and it writes
a reply. It is the other half of a RAG app (retrieval finds the facts, chat writes the
answer), and useful on its own for summaries, drafts and assistants.

## A one-shot prompt

`prompt` is the shortcut for a single question:

```rust
let ai = ctx.ai()?;
let reply = ai.prompt("Write a one-line summary of the borrow checker.").await?;
```

## A conversation

For multi-turn context — a system instruction, prior turns — build the messages and call
`chat`:

```rust
use rustali_ai::ChatMessage;

let reply = ai.chat(&[
    ChatMessage::system("You are a terse assistant. Answer in one sentence."),
    ChatMessage::user("What is ownership in Rust?"),
    ChatMessage::assistant("Each value has a single owner; it's dropped when the owner goes out of scope."),
    ChatMessage::user("And borrowing?"),
]).await?;
```

The three roles mirror how chat models are prompted:

| Role | Built with | Purpose |
|---|---|---|
| system | `ChatMessage::system(..)` | Instructions that steer the assistant. |
| user | `ChatMessage::user(..)` | What the person said. |
| assistant | `ChatMessage::assistant(..)` | A previous reply, for context in a multi-turn chat. |

`prompt(text)` is just `chat(&[ChatMessage::user(text)])`.

## Is chat available?

Chat and embeddings are **independent** — a handle may provide one, the other, or both. Check
before using it, and expect an error if no chat backend is configured:

```rust
if ai.has_chat() {
    let reply = ai.prompt("hello").await?;
}
```

With the default `mock` backend, `prompt` / `chat` return a **deterministic stub** that echoes
the last user message. That is enough to build and test your request flow, routes and
templates before a real model is in place — the shape of your code doesn't change when you
swap the stub for a real backend.

## Backends

Text generation is behind the `Chat` trait, so any backend — a local model, or a remote
provider like OpenAI or Anthropic — implements the same `generate(messages) -> String` and
plugs into the same `ai.chat(..)` / `ai.prompt(..)` calls. Today the shipped backend is the
deterministic `mock`; real generation backends slot in behind this trait without changing your
call sites.

::: tip Keep slow calls off the request path
Generation can take seconds. For anything user-facing that isn't streamed, consider running it
from a [queued job](../deeper/queues) and delivering the result out of band, the same way you
would a slow third-party API call.
:::
