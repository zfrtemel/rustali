//! `migrate`, `migrate:rollback`, `migrate:fresh`, `migrate:refresh`,
//! `migrate:reset`, `migrate:status`.

use rustali_core::{AppContext, Command, CommandFuture, Error, Result};
use rustali_orm::DbExt;

use crate::runner::MigrationRunner;

/// Parse `--step N` / `--steps N` from command arguments.
fn parse_steps(args: &[String]) -> Result<Option<u32>> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--step" || arg == "--steps" {
            let value = iter
                .next()
                .ok_or_else(|| Error::bad_request(format!("{arg} needs a number")))?;
            return value
                .parse::<u32>()
                .map(Some)
                .map_err(|_| Error::bad_request(format!("{arg} expects a number, got `{value}`")));
        }
        if let Some(value) = arg
            .strip_prefix("--step=")
            .or_else(|| arg.strip_prefix("--steps="))
        {
            return value.parse::<u32>().map(Some).map_err(|_| {
                Error::bad_request(format!("--step expects a number, got `{value}`"))
            });
        }
    }
    Ok(None)
}

fn runner(ctx: &AppContext) -> Result<std::sync::Arc<MigrationRunner>> {
    ctx.service::<MigrationRunner>()
}

/// Apply pending migrations.
pub struct Migrate;

impl Command for Migrate {
    fn name(&self) -> &'static str {
        "migrate"
    }

    fn about(&self) -> &'static str {
        "Apply pending database migrations"
    }

    fn run<'a>(&'a self, ctx: &'a AppContext, args: Vec<String>) -> CommandFuture<'a> {
        Box::pin(async move {
            let steps = parse_steps(&args)?;
            let db = ctx.db()?;
            runner(ctx)?.0.up(db.conn(), steps).await?;
            println!("Migrations applied.");
            Ok(())
        })
    }
}

/// Roll back the last batch (or `--step N`).
pub struct Rollback;

impl Command for Rollback {
    fn name(&self) -> &'static str {
        "migrate:rollback"
    }

    fn about(&self) -> &'static str {
        "Roll back the last migration (or --step N)"
    }

    fn run<'a>(&'a self, ctx: &'a AppContext, args: Vec<String>) -> CommandFuture<'a> {
        Box::pin(async move {
            let steps = parse_steps(&args)?.or(Some(1));
            let db = ctx.db()?;
            runner(ctx)?.0.down(db.conn(), steps).await?;
            println!("Rolled back.");
            Ok(())
        })
    }
}

/// Drop all tables and migrate from scratch.
pub struct Fresh;

impl Command for Fresh {
    fn name(&self) -> &'static str {
        "migrate:fresh"
    }

    fn about(&self) -> &'static str {
        "Drop all tables and re-run every migration"
    }

    fn run<'a>(&'a self, ctx: &'a AppContext, _args: Vec<String>) -> CommandFuture<'a> {
        Box::pin(async move {
            guard_production(ctx)?;
            let db = ctx.db()?;
            runner(ctx)?.0.fresh(db.conn()).await?;
            println!("Database rebuilt from scratch.");
            Ok(())
        })
    }
}

/// Roll back everything, then migrate.
pub struct Refresh;

impl Command for Refresh {
    fn name(&self) -> &'static str {
        "migrate:refresh"
    }

    fn about(&self) -> &'static str {
        "Roll back every migration, then re-run them"
    }

    fn run<'a>(&'a self, ctx: &'a AppContext, _args: Vec<String>) -> CommandFuture<'a> {
        Box::pin(async move {
            guard_production(ctx)?;
            let db = ctx.db()?;
            runner(ctx)?.0.refresh(db.conn()).await?;
            println!("Database refreshed.");
            Ok(())
        })
    }
}

/// Roll back everything.
pub struct Reset;

impl Command for Reset {
    fn name(&self) -> &'static str {
        "migrate:reset"
    }

    fn about(&self) -> &'static str {
        "Roll back every migration"
    }

    fn run<'a>(&'a self, ctx: &'a AppContext, _args: Vec<String>) -> CommandFuture<'a> {
        Box::pin(async move {
            guard_production(ctx)?;
            let db = ctx.db()?;
            runner(ctx)?.0.down(db.conn(), None).await?;
            println!("All migrations rolled back.");
            Ok(())
        })
    }
}

/// Show applied / pending migrations.
pub struct Status;

impl Command for Status {
    fn name(&self) -> &'static str {
        "migrate:status"
    }

    fn about(&self) -> &'static str {
        "Show which migrations are applied"
    }

    fn run<'a>(&'a self, ctx: &'a AppContext, _args: Vec<String>) -> CommandFuture<'a> {
        Box::pin(async move {
            let db = ctx.db()?;
            let statuses = runner(ctx)?.0.status(db.conn()).await?;
            if statuses.is_empty() {
                println!("No migrations found.");
            }
            for status in statuses {
                let mark = if status.applied { "applied" } else { "pending" };
                println!("{mark:>8}  {}", status.name);
            }
            Ok(())
        })
    }
}

/// Destructive commands refuse to run in production unless `--force` is passed.
fn guard_production(ctx: &AppContext) -> Result<()> {
    if ctx.config().is_production() && std::env::var_os("RUSTALI_FORCE").is_none() {
        return Err(Error::bad_request(
            "refusing to run a destructive migration command in production; \
             set RUSTALI_FORCE=1 if you really mean it",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_parsing() {
        let args = |s: &[&str]| s.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
        assert_eq!(parse_steps(&args(&[])).ok(), Some(None));
        assert_eq!(parse_steps(&args(&["--step", "3"])).ok(), Some(Some(3)));
        assert_eq!(parse_steps(&args(&["--steps=2"])).ok(), Some(Some(2)));
        assert!(parse_steps(&args(&["--step", "x"])).is_err());
        assert!(parse_steps(&args(&["--step"])).is_err());
    }
}
