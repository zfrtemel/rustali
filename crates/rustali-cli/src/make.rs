//! `rustali make:<thing> <Name>` generators.

use std::path::Path;

use anyhow::{Result, bail};
use heck::{ToSnakeCase, ToUpperCamelCase};
use minijinja::context;

use crate::{generate, templates};

/// Declare `name` in `dir`'s `mod.rs` (via [`generate::ensure_module`]). When this is the
/// first file in a new kind directory, remind the user to add `mod <kind>;` to `main.rs`
/// once — the generator deliberately doesn't edit `main.rs` (it can't tell bin from lib).
fn declare(dir: &Path, name: &str, pub_use: bool) -> Result<()> {
    let is_new_dir = !dir.join("mod.rs").exists();
    generate::ensure_module(dir, name, pub_use)?;
    if is_new_dir && let Some(kind) = dir.file_name().and_then(|s| s.to_str()) {
        println!("  next: add `mod {kind};` to src/main.rs");
    }
    Ok(())
}

/// Run a `make:<kind>` command. `kind` is the part after the colon.
///
/// One match arm per generator; kept in a single function because the arms are short and
/// share the derived names and `force` flag.
#[allow(clippy::too_many_lines)]
pub(crate) fn run(kind: &str, args: &[String]) -> Result<()> {
    let force = args.iter().any(|a| a == "--force" || a == "-f");
    let name = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!("make:{kind} needs a name, e.g. `rustali make:{kind} Post`")
        })?;

    let root = generate::project_root();
    let src = root.join("src");

    // Derived names used across templates.
    let type_name = name.to_upper_camel_case();
    let file_name = name.to_snake_case();

    match kind {
        "controller" => {
            let ctx = context! { name => type_name, module => file_name };
            let dir = src.join("controllers");
            generate::write_rendered(
                &dir.join(format!("{file_name}.rs")),
                templates::CONTROLLER,
                &ctx,
                force,
            )?;
            // Controllers auto-register via inventory, so the aggregator only links them.
            declare(&dir, &file_name, false)?;
        }
        "model" => {
            let table = format!("{file_name}s");
            let ctx = context! { name => type_name, table => table };
            let dir = src.join("models");
            generate::write_rendered(
                &dir.join(format!("{file_name}.rs")),
                templates::MODEL,
                &ctx,
                force,
            )?;
            // Models are referenced by path (`models::Post`), so re-export them.
            declare(&dir, &file_name, true)?;
        }
        "migration" => {
            let stamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let module = format!("m{stamp}_{file_name}");
            let ctx = context! { table => file_name };
            let dir = src.join("migrations");
            generate::write_rendered(
                &dir.join(format!("{module}.rs")),
                templates::MIGRATION,
                &ctx,
                force,
            )?;
            // Link the module; the Migrator's `migrations()` vec is still edited by hand.
            generate::ensure_module(&dir, &module, false)?;
            println!("  next: list `{module}::Migration` in your Migrator");
        }
        "job" => {
            let ctx = context! { name => type_name, job_name => file_name };
            let dir = src.join("jobs");
            generate::write_rendered(
                &dir.join(format!("{file_name}.rs")),
                templates::JOB,
                &ctx,
                force,
            )?;
            declare(&dir, &file_name, true)?;
        }
        "mail" => {
            let ctx = context! { name => type_name };
            let dir = src.join("mail");
            generate::write_rendered(
                &dir.join(format!("{file_name}.rs")),
                templates::MAIL,
                &ctx,
                force,
            )?;
            declare(&dir, &file_name, true)?;
        }
        "middleware" => {
            let ctx = context! { name => type_name };
            let dir = src.join("middleware");
            generate::write_rendered(
                &dir.join(format!("{file_name}.rs")),
                templates::MIDDLEWARE,
                &ctx,
                force,
            )?;
            declare(&dir, &file_name, true)?;
        }
        "policy" => {
            let ctx = context! { name => type_name, ability => file_name };
            let dir = src.join("policies");
            generate::write_rendered(
                &dir.join(format!("{file_name}.rs")),
                templates::POLICY,
                &ctx,
                force,
            )?;
            declare(&dir, &file_name, true)?;
        }
        "test" => {
            let ctx = context! { name => file_name };
            generate::write_rendered(
                &root.join("tests").join(format!("{file_name}.rs")),
                templates::TEST,
                &ctx,
                force,
            )?;
        }
        other => bail!(
            "unknown generator `make:{other}`. Try controller, model, migration, job, mail, \
             middleware, policy or test."
        ),
    }
    Ok(())
}
