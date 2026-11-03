//! Forward application commands (`serve`, `migrate`, `routes`, ...) to the project's
//! own binary.
//!
//! These commands run the user's code — their entities, migrations and controllers —
//! so they live in the application binary, not in this CLI. Rather than make the user
//! type `cargo run -- <command>`, `rustali <command>` finds the enclosing project and
//! runs it for them (the same convenience Laravel's `artisan` offers).

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};

/// App commands the CLI knows to forward. Anything else is a genuine unknown command.
///
/// `migrate:*` and other colon-suffixed variants are matched by prefix, so only the
/// base word needs to appear here.
const APP_COMMANDS: &[&str] = &["serve", "migrate", "routes", "queue", "schedule", "db"];

/// Whether `command` is an application command this CLI should forward.
pub(crate) fn is_app_command(command: &str) -> bool {
    // `migrate:diff`, `queue:work`, `db:seed`, ... share the base word before the colon.
    let base = command.split(':').next().unwrap_or(command);
    APP_COMMANDS.contains(&base)
}

/// Run `command` (plus `rest`) as `cargo run -- <command> <rest...>` from the project
/// root, inheriting stdio so output and exit status pass straight through.
pub(crate) fn run(command: &str, rest: &[String]) -> Result<()> {
    let root = project_root().with_context(|| {
        format!(
            "`{command}` is an application command, but this doesn't look like a Rustali \
             project (no Cargo.toml with a `rustali` dependency found here or in a parent \
             directory). Run it from inside a project."
        )
    })?;

    let status = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg(command)
        .args(rest)
        .current_dir(&root)
        .status()
        .context("failed to run `cargo` — is it installed and on PATH?")?;

    if !status.success() {
        // cargo already printed the error; exit non-zero without a second message.
        bail!("`{command}` exited with a non-zero status");
    }
    Ok(())
}

/// Walk up from the current directory for the nearest `Cargo.toml` that depends on
/// `rustali` — the application crate whose binary owns these commands.
fn project_root() -> Option<PathBuf> {
    let start = std::env::current_dir().ok()?;
    for dir in start.ancestors() {
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file() && manifest_uses_rustali(&manifest) {
            return Some(dir.to_path_buf());
        }
    }
    None
}

/// A cheap check that the manifest names `rustali` as a dependency, so we don't try to
/// forward commands from an unrelated Rust project.
fn manifest_uses_rustali(manifest: &Path) -> bool {
    std::fs::read_to_string(manifest).is_ok_and(|text| {
        text.lines()
            .any(|line| line.trim_start().starts_with("rustali ") || line.trim_start() == "rustali")
            || text.contains("rustali =")
    })
}
