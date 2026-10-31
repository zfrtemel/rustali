//! `cargo xtask <command>` — repository automation that is too awkward for shell.
//!
//! Commands grow with the project: `ci` (fmt + clippy + test + doc), `layering`
//! (checks the crate dependency direction from ADR-0001), `starter-sync`.

use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask", about = "Rustali repository automation")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the same checks as CI, in order, stopping at the first failure.
    Ci,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Cmd::Ci => ci(),
    }
}

fn ci() -> Result<()> {
    run(&["fmt", "--all", "--check"])?;
    run(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ])?;
    run(&["test", "--workspace", "--all-features"])?;
    run(&["doc", "--workspace", "--all-features", "--no-deps"])?;
    Ok(())
}

fn run(args: &[&str]) -> Result<()> {
    eprintln!("$ cargo {}", args.join(" "));
    let status = Command::new("cargo")
        .args(args)
        .status()
        .context("spawning cargo")?;
    if !status.success() {
        bail!("`cargo {}` failed with {status}", args.join(" "));
    }
    Ok(())
}
