//! The `migrate:diff` command: entities vs snapshot ⇒ a new migration file.

use std::path::Path;

use chrono::Utc;
use rustali_core::{AppContext, Command, CommandFuture, Error, Result};
use rustali_orm::{desired_schema, schema::SchemaChange};

use crate::{generate::migration_source, snapshot};

/// `migrate:diff "name"` — generate a migration from entity changes.
///
/// Compares the schema built from every `#[derive(Entity)]` in the binary against
/// `db/schema.snapshot.json`, writes a reviewed-by-you migration for the difference,
/// updates the snapshot, and reminds you to register the module in your `Migrator`.
pub struct DiffCommand;

impl Command for DiffCommand {
    fn name(&self) -> &'static str {
        "migrate:diff"
    }

    fn about(&self) -> &'static str {
        "Generate a migration from entity changes (code-first)"
    }

    fn run<'a>(&'a self, _ctx: &'a AppContext, args: Vec<String>) -> CommandFuture<'a> {
        Box::pin(async move { run_diff(&args) })
    }
}

fn run_diff(args: &[String]) -> Result<()> {
    let allow_destructive = args.iter().any(|a| a == "--allow-destructive");
    let name = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| "update".to_owned());

    let snapshot_path = Path::new(snapshot::DEFAULT_PATH);
    let current = snapshot::load(snapshot_path)?;
    let desired = desired_schema();
    let changes = current.diff(&desired);

    if changes.is_empty() {
        println!("Schema is up to date; nothing to generate.");
        return Ok(());
    }

    report(&changes);

    let destructive: Vec<&SchemaChange> = changes.iter().filter(|c| c.is_destructive()).collect();
    if !destructive.is_empty() && !allow_destructive {
        return Err(Error::bad_request(format!(
            "{} destructive change(s) detected (drop / type change / NOT NULL). Re-run \
             with --allow-destructive once you have reviewed them.",
            destructive.len()
        )));
    }

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let slug = slugify(&name);
    let module = format!("m{timestamp}_{slug}");
    let source = migration_source(&module, &changes);

    let dir = snapshot::migrations_dir(snapshot_path);
    std::fs::create_dir_all(&dir).map_err(Error::internal)?;
    let file = dir.join(format!("{module}.rs"));
    std::fs::write(&file, source).map_err(Error::internal)?;

    snapshot::save(snapshot_path, &desired)?;

    println!("\nWrote {}", file.display());
    println!("Updated {}", snapshot_path.display());
    println!("\nNext: add it to your Migrator:");
    println!("    mod {module};");
    println!("    // ... Box::new({module}::Migration) in migrations()");
    Ok(())
}

fn report(changes: &[SchemaChange]) {
    println!("Detected {} change(s):", changes.len());
    for change in changes {
        let mark = if change.is_destructive() {
            "  [!]"
        } else {
            "   - "
        };
        println!("{mark} {}", change.summary());
    }
}

/// `Add phone to users!` → `add_phone_to_users`.
fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_underscore = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.extend(ch.to_lowercase());
            last_underscore = false;
        } else if !last_underscore && !out.is_empty() {
            out.push('_');
            last_underscore = true;
        }
    }
    let trimmed = out.trim_end_matches('_');
    if trimmed.is_empty() {
        "update".to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_makes_snake_case() {
        assert_eq!(slugify("Add phone to users!"), "add_phone_to_users");
        assert_eq!(slugify("create-posts"), "create_posts");
        assert_eq!(slugify("  "), "update");
        assert_eq!(slugify("CreateUsers"), "createusers");
    }
}
