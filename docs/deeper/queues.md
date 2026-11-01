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

`RETRY` defaults to `Retry::NONE` (one attempt). `Retry::times(n, backoff)` retries `n`
more times with a fixed delay.

## Registering jobs and the worker

The worker needs to know each job type by name (to deserialise it), so build a `Worker`
listing them and pass it to `QueueProvider`:

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

## Running the worker

Process queued jobs with the console command:

```bash
cargo run -- queue:work        # drains the queue until Ctrl-C
```

In development you can run it alongside `cargo run`; in production run it as a separate
process.

## What ships today

The built-in queue is **in-process**: dispatched jobs go over a channel that
`queue:work` drains. Jobs survive the request but **not a process restart**. A durable
database/Redis backend plugs in behind the same `Queue` trait later — your job code does not
change. For now, do not use it for work you cannot afford to lose across a restart.

> `rustali make:job <Name>` scaffolds a job.
