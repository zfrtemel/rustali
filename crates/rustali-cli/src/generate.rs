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

/// Ensure `dir/mod.rs` declares `mod <name>;`, creating the aggregator if absent and
/// appending the line if missing. When `pub_use` is set, also ensure `pub use <name>::*;`.
///
/// Idempotent: re-running leaves an already-declared module untouched. Matching is on
/// whole trimmed lines, so `mod post;` never spuriously matches inside `mod posture;`.
pub(crate) fn ensure_module(dir: &Path, name: &str, pub_use: bool) -> Result<()> {
    let modrs = dir.join("mod.rs");
    let existing = std::fs::read_to_string(&modrs).unwrap_or_default();
    let has_line = |needle: &str| existing.lines().any(|line| line.trim() == needle);

    let mut additions = String::new();
    let mod_line = format!("mod {name};");
    if !has_line(&mod_line) {
        additions.push_str(&mod_line);
        additions.push('\n');
    }
    if pub_use {
        let use_line = format!("pub use {name}::*;");
        if !has_line(&use_line) {
            additions.push_str(&use_line);
            additions.push('\n');
        }
    }

    if additions.is_empty() {
        return Ok(());
    }

    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let updated = format!("{existing}{additions}");
    std::fs::write(&modrs, updated).with_context(|| format!("writing {}", modrs.display()))?;
    let verb = if existing.is_empty() {
        "created"
    } else {
        "updated"
    };
    println!("  {} {}", style(verb).green(), modrs.display());
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    /// A unique temp directory for one test, cleaned up on drop.
    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("rustali-gen-test-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn ensure_module_creates_aggregator() {
        let tmp = TempDir::new("create");
        ensure_module(&tmp.0, "post", true).unwrap();
        let text = std::fs::read_to_string(tmp.0.join("mod.rs")).unwrap();
        assert_eq!(text, "mod post;\npub use post::*;\n");
    }

    #[test]
    fn ensure_module_without_pub_use() {
        let tmp = TempDir::new("nopubuse");
        ensure_module(&tmp.0, "home", false).unwrap();
        let text = std::fs::read_to_string(tmp.0.join("mod.rs")).unwrap();
        assert_eq!(text, "mod home;\n");
    }

    #[test]
    fn ensure_module_is_idempotent() {
        let tmp = TempDir::new("idem");
        ensure_module(&tmp.0, "post", true).unwrap();
        ensure_module(&tmp.0, "post", true).unwrap();
        let text = std::fs::read_to_string(tmp.0.join("mod.rs")).unwrap();
        assert_eq!(text, "mod post;\npub use post::*;\n", "no duplicate lines");
    }

    #[test]
    fn ensure_module_appends_to_existing() {
        let tmp = TempDir::new("append");
        ensure_module(&tmp.0, "home", false).unwrap();
        ensure_module(&tmp.0, "about", false).unwrap();
        let text = std::fs::read_to_string(tmp.0.join("mod.rs")).unwrap();
        assert_eq!(text, "mod home;\nmod about;\n");
    }

    #[test]
    fn ensure_module_no_substring_false_match() {
        let tmp = TempDir::new("substr");
        ensure_module(&tmp.0, "posture", false).unwrap();
        // `mod post;` must still be added — it is not the same line as `mod posture;`.
        ensure_module(&tmp.0, "post", false).unwrap();
        let text = std::fs::read_to_string(tmp.0.join("mod.rs")).unwrap();
        assert_eq!(text, "mod posture;\nmod post;\n");
    }
}
