# Queues & Jobs

Move slow work (email, image processing, calling a third-party API) off the request path.
Define it as a `Job`, dispatch it from a handler, and a worker runs it in the background.

## Defining a job

A job is a serialisable struct implementing `Job`. It names itself, optionally sets a retry
policy, and does its work in `handle` — which gets the full `AppContext`.

```rust
use rustali::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SendWelcome {
    user_id: i64,
}

#[async_trait]
impl Job for SendWelcome {
    const NAME: &'static str = "send_welcome";          // unique, routes the job back
    const RETRY: Retry = Retry::times(3, Duration::from_secs(5));   // optional

    async fn handle(&self, ctx: &AppContext) -> Result<()> {
        let user = User::find_or_404(&*ctx.db()?, self.user_id).await?;
        ctx.mailer()?.send(&user.email, &Welcome { name: user.name }).await
    }
}
```

### Retries

`RETRY` controls what happens when `handle` returns an `Err`:

| Value | Attempts | On final failure |
|---|---|---|
| `Retry::NONE` (default) | 1 | logged and dropped |
| `Retry::times(n, backoff)` | 1 + `n` | logged and dropped |

`times(n, ...)` means `n` retries **beyond** the first attempt, so `Retry::times(3, ...)` is
up to four runs in total. The backoff is a **fixed** delay between attempts (exponential
backoff may come later). A job that exhausts its retries is logged and dropped — there is no
dead-letter store yet — so treat retries as a best-effort cushion, not a guarantee.

### Write jobs to be safe to retry

Because a job can run more than once (a retry, or — with a durable backend later — an
at-least-once delivery), make `handle` **idempotent** where it matters: guard against
sending the same email twice, charging twice, or inserting a duplicate row. A job that is
safe to run again is a job you never have to think hard about when it retries.

## Registering jobs and the worker

When a job is dispatched it is **serialised** (it is `Serialize`), tagged with its `NAME`,
and put on the queue. The worker reads the name to pick the right type and **deserialise** it
back — which is why `NAME` must be unique across your jobs, and why changing a job's fields
is a format change for anything already queued. Build a `Worker` listing every job type so
the worker knows how to route each name, and pass it to `QueueProvider`:

```rust
let worker = Worker::builder()
    .register::<SendWelcome>()
    .register::<GenerateThumbnail>()
    .build();

App::builder()
    .provider(DatabaseProvider)
    .provider(MailProvider)
    .provider(QueueProvider::new(worker))
    .controllers()
    .run()
    .await
```

## Dispatching

From a handler, reach the queue with `ctx.queue()` (or `Inject<SharedQueue>`) and dispatch:

```rust
#[post("/signup")]
async fn signup(ctx: Ctx, db: Db, /* ... */) -> Result<Redirect> {
    let user = /* create user */;
    ctx.queue()?.dispatch(SendWelcome { user_id: user.id }).await?;
    Ok(Redirect::to("/dashboard"))
}
```

A job type the worker does not know about cannot be deserialised: the worker reports a clear
"no handler registered for job `<name>`" error rather than running the wrong code. So adding
a new job type means adding one `.register::<T>()` line.

## Running the worker

Process queued jobs with the console command (registered by `QueueProvider`):

```bash
cargo run -- queue:work        # drains the queue until Ctrl-C
```

In development you can run it alongside `cargo run`; in production run it as a separate
process. The worker and the web process share the same `App::builder()`, so a job's `handle`
has the exact same context — database, mailer, your services — as a request handler.

## What ships today

The built-in queue is **in-process**: dispatched jobs go over a channel that
`queue:work` drains. Jobs survive the request but **not a process restart**. A durable
database/Redis backend plugs in behind the same `Queue` trait later — your job code does not
change. For now, do not use it for work you cannot afford to lose across a restart.

> `rustali make:job <Name>` scaffolds a job.
