//! The `rustali` command-line tool.
//!
//! `rustali new <name>` scaffolds a fresh application; `rustali make:<thing> <Name>`
//! generates a source file from a template (Laravel's `make:*` convention, colon and
//! all). Everything else (serve, migrate, routes, queue:work, ...) is provided by the
//! application binary — run those with `cargo run -- <command>` inside a project.

mod generate;
mod make;
mod new;
mod templates;

use std::process::ExitCode;

use console::style;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{} {err}", style("error:").red().bold());
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> anyhow::Result<()> {
    let Some(command) = args.first().map(String::as_str) else {
        print_help();
        return Ok(());
    };
    let rest = &args[1..];

    match command {
        "new" => new::run(rest),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        "--version" | "-V" => {
            println!("rustali {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        other if other.starts_with("make:") => make::run(other.trim_start_matches("make:"), rest),
        other => {
            anyhow::bail!(
                "unknown command `{other}`.\n       In a project, app commands like \
                 serve/migrate/routes run via `cargo run -- {other}`.\n       \
                 See `rustali help` for scaffolding commands."
            )
        }
    }
}

fn print_help() {
    println!(
        "{}\n\n\
         {}\n  rustali new <name> [--preset api|fullstack|minimal] [--with cache,mail]\n  \
         rustali make:controller <Name>  Generate a controller\n  \
         rustali make:model <Name>        Generate a #[derive(Entity)] model\n  \
         rustali make:migration <name>    Generate an empty migration\n  \
         rustali make:job <Name>          Generate a background job\n  \
         rustali make:mail <Name>         Generate a mailable\n  \
         rustali make:middleware <Name>   Generate a tower middleware\n  \
         rustali make:policy <Name>       Generate an authorization policy\n  \
         rustali make:test <name>         Generate an integration test\n\n\
         {}\n  Inside a project, run the app's own commands with cargo:\n    \
         cargo run -- serve | migrate | migrate:diff \"msg\" | routes | queue:work",
        style("The Rustali web framework CLI").bold(),
        style("Scaffolding:").bold(),
        style("App commands:").bold(),
    );
}
