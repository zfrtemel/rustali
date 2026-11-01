//! `rustali make:<thing> <Name>` generators.

use anyhow::{Result, bail};
use heck::{ToSnakeCase, ToUpperCamelCase};
use minijinja::context;

use crate::{generate, templates};

/// Run a `make:<kind>` command. `kind` is the part after the colon.
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
            generate::write_rendered(
                &src.join("controllers").join(format!("{file_name}.rs")),
                templates::CONTROLLER,
                &ctx,
                force,
            )?;
        }
        "model" => {
            let table = format!("{file_name}s");
            let ctx = context! { name => type_name, table => table };
            generate::write_rendered(
                &src.join("models").join(format!("{file_name}.rs")),
                templates::MODEL,
                &ctx,
                force,
            )?;
        }
        "migration" => {
            let stamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let module = format!("m{stamp}_{file_name}");
            let ctx = context! { table => file_name };
            generate::write_rendered(
                &src.join("migrations").join(format!("{module}.rs")),
                templates::MIGRATION,
                &ctx,
                force,
            )?;
            println!("  next: add `mod {module};` and list it in your Migrator");
        }
        "job" => {
            let ctx = context! { name => type_name, job_name => file_name };
            generate::write_rendered(
                &src.join("jobs").join(format!("{file_name}.rs")),
                templates::JOB,
                &ctx,
                force,
            )?;
        }
        "mail" => {
            let ctx = context! { name => type_name };
            generate::write_rendered(
                &src.join("mail").join(format!("{file_name}.rs")),
                templates::MAIL,
                &ctx,
                force,
            )?;
        }
        "middleware" => {
            let ctx = context! { name => type_name };
            generate::write_rendered(
                &src.join("middleware").join(format!("{file_name}.rs")),
                templates::MIDDLEWARE,
                &ctx,
                force,
            )?;
        }
        "policy" => {
            let ctx = context! { name => type_name, ability => file_name };
            generate::write_rendered(
                &src.join("policies").join(format!("{file_name}.rs")),
                templates::POLICY,
                &ctx,
                force,
            )?;
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
