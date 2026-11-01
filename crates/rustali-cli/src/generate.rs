//! Rendering templates to files, safely.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use console::style;
use minijinja::Environment;

/// Render `template` with `context` and write it to `path`, refusing to overwrite an
/// existing file unless `force`. Creates parent directories. Prints a tidy line.
pub(crate) fn write_rendered(
    path: &Path,
    template: &str,
    context: &minijinja::Value,
    force: bool,
) -> Result<()> {
    if path.exists() && !force {
        bail!(
            "{} already exists (use --force to overwrite)",
            path.display()
        );
    }
    let rendered = render(template, context)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(path, rendered).with_context(|| format!("writing {}", path.display()))?;
    println!("  {} {}", style("created").green(), path.display());
    Ok(())
}

/// Render a template string with minijinja.
pub(crate) fn render(template: &str, context: &minijinja::Value) -> Result<String> {
    let mut env = Environment::new();
    env.add_template("t", template)
        .context("parsing template")?;
    let tmpl = env.get_template("t")?;
    tmpl.render(context).context("rendering template")
}

/// Write a non-templated file (verbatim), same overwrite rules.
pub(crate) fn write_file(path: &Path, contents: &str, force: bool) -> Result<()> {
    if path.exists() && !force {
        bail!(
            "{} already exists (use --force to overwrite)",
            path.display()
        );
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(path, contents).with_context(|| format!("writing {}", path.display()))?;
    println!("  {} {}", style("created").green(), path.display());
    Ok(())
}

/// The directory a `make:*` command writes into: the project root if we are in one
/// (a `Cargo.toml` with `[dependencies] rustali` nearby), else the current directory.
#[must_use]
pub(crate) fn project_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        if dir.join("Cargo.toml").exists() && dir.join("src").is_dir() {
            return dir;
        }
        if !dir.pop() {
            return std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        }
    }
}
