# Mail

Define a message by implementing `Mailable`, then send it through the registered mailer.
Three transports cover development, tests and production.

## Setup

Add `MailProvider`; configure it in `.env`:

```rust
App::builder().provider(MailProvider).controllers().run().await
```

```bash
# .env
MAIL_TRANSPORT=log                 # log | memory | smtp
MAIL_FROM=no-reply@example.com
MAIL_URL=smtp://localhost:1025     # used when MAIL_TRANSPORT=smtp
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

### The Mailable trait

| Method | Required | Returns |
|---|---|---|
| `subject(&self)` | yes | `String` — the subject line. |
| `html(&self)` | yes | `String` — the HTML body. |
| `text(&self)` | no (default `None`) | `Option<String>` — a plain-text alternative. |
| `reply_to(&self)` | no (default `None`) | `Option<String>` — a `Reply-To` address. |

Provide `text` when you want a plain-text part alongside the HTML (good for deliverability
and text-only clients); leave it out and the message is HTML-only.

### Rendering the body with a template

The body is just a `String`, so render it however you like — including with an
[Askama template](../basics/views). Any `askama::Template` has a `render()` that returns a
`String`, so a mailable can own a template and render it in `html`:

```rust
use askama::Template;

#[derive(Template)]
#[template(path = "mail/welcome.html")]
struct WelcomeHtml<'a> { name: &'a str }

struct Welcome { name: String }

impl Mailable for Welcome {
    fn subject(&self) -> String { "Welcome!".into() }
    fn html(&self) -> String {
        WelcomeHtml { name: &self.name }.render().unwrap_or_default()
    }
}
```

This keeps email markup in a template file under `templates/`, next to your page views,
rather than inline in Rust strings.

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

## Choosing a transport per environment

The transport is a config value, so each environment picks its own without a code change:

- **development** — `log`, so you see the rendered message in your terminal with no SMTP
  server to run.
- **tests** — `memory`, so you can assert on exactly what was sent (see below).
- **production** — `smtp`, pointed at your real mail service via `url`.

Because the mailer sits behind one `send` call, swapping transports never touches the code
that builds and sends messages.

## Testing mail

Set `transport = "memory"` (or construct a `MemoryTransport`) and assert on what was sent.
A `MemoryTransport` records every delivered message:

```rust
let transport = MemoryTransport::new();
// ... run the code that sends ...
assert_eq!(transport.count(), 1);
let sent = transport.sent();                 // Vec<Email>
assert_eq!(sent[0].to, "ada@example.com");
assert!(sent[0].subject.contains("Welcome"));
```

`sent()` returns every delivered `Email` (with `from`, `to`, `subject`, `html` and the
optional `text`); `count()` is the number delivered.

## Scope

The mailer covers the common path: a single recipient, an HTML body with an optional
plain-text part, and an optional `Reply-To`. It does not model CC/BCC, multiple recipients
or attachments — if you need those, reach for `lettre` (which the SMTP transport is built
on) directly.
