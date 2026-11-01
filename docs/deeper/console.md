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

The binary dispatches on the first argument: no argument (or `serve`) runs the server,
`help` lists the commands, and any other word is looked up among the registered commands —
an unknown name is a clear error. So the set of commands your binary understands is exactly
what its providers and `.command(...)` calls registered — nothing more, nothing hidden.

## Built-in commands

Which commands exist depends on which providers you added:

| Command | From | Does |
|---|---|---|
| `serve` | always | Start the HTTP server (the default). |
| `routes` | always | List controller routes with their middleware. |
| `help` | always | List commands. |
| `migrate`, `migrate:diff`, `migrate:status`, `migrate:rollback`, `migrate:fresh`, `migrate:refresh`, `migrate:reset` | `.migrations::<M>()` | Database migrations. |
| `queue:work` | `QueueProvider` | Process queued jobs. |

`help` lists every registered command with its one-line `about`, so the quickest way to see
what a given build supports is to run it:

```bash
cargo run -- help
```

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

### Arguments

`run` receives everything after the command name as a `Vec<String>` — the binary name and
the command word are already stripped. For `cargo run -- report:send a@x b@x`, `args` is
`["a@x", "b@x"]`. Parse it yourself (or hand it to a crate like `clap`); the framework does
not impose a flag syntax on your commands.

### Errors and exit codes

`run` returns `Result<()>`, and that result flows up to `main`. Because `main` is
`-> Result<()>`, returning an `Err` prints it and exits with a non-zero status — so a failed
command fails the shell too (and your CI), without any manual `std::process::exit`. Use `?`
freely:

```rust
fn run<'a>(&'a self, ctx: &'a AppContext, args: Vec<String>) -> CommandFuture<'a> {
    Box::pin(async move {
        let to = args.first().ok_or_else(|| Error::bad_request("usage: report:send <email>"))?;
        let db = ctx.db()?;
        // ...
        Ok(())
    })
}
```

### Full context

Because a command runs through the same `App::builder()`, it has the full context —
database, cache, mailer, your services — exactly as a request handler does. Providers run
before the command, so everything they register is available through `ctx`. The difference
from a handler is only the entry point: there is no HTTP request, just `ctx` and `args`.

## Scaffolding

`rustali make:controller`, `make:model`, `make:migration`, `make:job`, `make:mail`,
`make:middleware`, `make:policy` and `make:test` are commands of the standalone `rustali`
CLI (not the app binary) — they generate source files. See
[Project Structure](../getting-started/structure).
