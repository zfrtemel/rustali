# Mail

Define a message by implementing `Mailable`, then send it through the registered mailer.
Three transports cover development, tests and production.

## Setup

Add `MailProvider`; configure in `config/mail.toml`:

```rust
App::builder().provider(MailProvider).controllers().run().await
```

```toml
# config/mail.toml
transport = "log"                 # log | memory | smtp
from = "no-reply@example.com"
url = "smtp://localhost:1025"     # used when transport = "smtp"
```

- **log** (default) — writes each message to `tracing` instead of sending. See what would go
  out without an SMTP server.
- **memory** — records messages in memory for test assertions.
- **smtp** — real delivery via SMTP (needs the `smtp` feature, on by default).

## Defining a message

```rust
use rustali::prelude::*;

struct Welcome {
    name: String,
}

impl Mailable for Welcome {
    fn subject(&self) -> String { "Welcome!".into() }
    fn html(&self) -> String { format!("<h1>Hi {}</h1>", self.name) }
    // fn text(&self) -> Option<String> { ... }       // optional plain-text alternative
    // fn reply_to(&self) -> Option<String> { ... }   // optional Reply-To
}
```

The body is just a `String`, so render it however you like — including with an
[Askama template](../basics/views) rendered to a string.

## Sending

Reach the mailer with `ctx.mailer()` (or `Inject<SharedMailer>`) and send to a recipient;
the `from` defaults to the configured address:

```rust
#[post("/signup")]
async fn signup(ctx: Ctx, db: Db, /* ... */) -> Result<Redirect> {
    let user = /* create user */;
    ctx.mailer()?.send(&user.email, &Welcome { name: user.name.clone() }).await?;
    Ok(Redirect::to("/dashboard"))
}
```

Use `send_from(from, to, &mail)` to override the sender for one message.

## Sending in the background

Email is slow; keep it off the request path by sending from a [job](queues):

```rust
#[derive(Serialize, Deserialize)]
struct SendWelcome { user_id: i64 }

#[async_trait]
impl Job for SendWelcome {
    const NAME: &'static str = "send_welcome";
    async fn handle(&self, ctx: &AppContext) -> Result<()> {
        let user = User::find_or_404(&*ctx.db()?, self.user_id).await?;
        ctx.mailer()?.send(&user.email, &Welcome { name: user.name }).await
    }
}

// in the handler:
ctx.queue()?.dispatch(SendWelcome { user_id: user.id }).await?;
```

## Testing mail

Set `transport = "memory"` (or construct a `MemoryTransport`) and assert on what was sent —
`sent()` returns every delivered `Email`, `count()` the number.
