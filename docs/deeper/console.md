# Console Commands

Your application binary is also its console. With no arguments it serves; with a command it
runs that instead. This is what `run()` does:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    App::builder().provider(DatabaseProvider).controllers().run().await
}
```

```bash
cargo run                      # serve
cargo run -- routes            # run the `routes` command
cargo run -- migrate           # run the `migrate` command
cargo run -- help              # list available commands
```

## Built-in commands

Which commands exist depends on which providers you added:

| Command | From | Does |
|---|---|---|
| `serve` | always | Start the HTTP server (the default). |
| `routes` | always | List controller routes with their middleware. |
| `help` | always | List commands. |
| `migrate`, `migrate:diff`, `migrate:status`, `migrate:rollback`, `migrate:fresh`, `migrate:refresh`, `migrate:reset` | `.migrations::<M>()` | Database migrations. |
| `queue:work` | `QueueProvider` | Process queued jobs. |

## Writing a command

Implement `Command`: a name, a one-line description, and an async `run` that receives the
`AppContext` and the remaining arguments.

```rust
use rustali::prelude::*;
use rustali::{Command, CommandFuture};

struct SendReport;

impl Command for SendReport {
    fn name(&self) -> &'static str { "report:send" }
    fn about(&self) -> &'static str { "Email the daily report" }

    fn run<'a>(&'a self, ctx: &'a AppContext, args: Vec<String>) -> CommandFuture<'a> {
        Box::pin(async move {
            let db = ctx.db()?;
            // ... build and send the report ...
            println!("sent to {} recipients", args.len());
            Ok(())
        })
    }
}
```

Register it on the builder (or from a provider with `registry.command(...)`):

```rust
App::builder()
    .command(SendReport)
    .controllers()
    .run()
    .await
```

```bash
cargo run -- report:send alice@example.com bob@example.com
```

Because a command runs through the same `App::builder()`, it has the full context —
database, cache, mailer, your services — exactly as a request handler does.

## Scaffolding

`rustali make:controller`, `make:model`, `make:migration`, `make:job`, `make:mail`,
`make:middleware`, `make:policy` and `make:test` are commands of the standalone `rustali`
CLI (not the app binary) — they generate source files. See
[Project Structure](../getting-started/structure).
