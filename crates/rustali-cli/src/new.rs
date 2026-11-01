//! `rustali new <name>` — scaffold a fresh application.
#![allow(clippy::needless_raw_string_hashes)]

use std::path::Path;

use anyhow::{Result, bail};
use console::style;
use minijinja::context;

use crate::generate::{write_file, write_rendered};

/// Scaffold a new project. `args` is everything after `new`.
pub(crate) fn run(args: &[String]) -> Result<()> {
    let api_only = args.iter().any(|a| a == "--api");
    let force = args.iter().any(|a| a == "--force");
    let name = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("usage: rustali new <name> [--api]"))?;

    let dir = Path::new(&name);
    if dir.exists() && !force {
        bail!("directory `{name}` already exists (use --force to write into it)");
    }

    // The crate name must be a valid Rust identifier-ish; keep it simple.
    let crate_name = name.replace(['-', ' '], "_");
    let ctx = context! { name => name, crate_name => crate_name, api => api_only };

    println!("{} {name}", style("Creating").green().bold());

    write_rendered(&dir.join("Cargo.toml"), CARGO_TOML, &ctx, force)?;
    write_file(&dir.join(".gitignore"), GITIGNORE, force)?;
    write_file(&dir.join(".env"), ENV, force)?;
    write_rendered(&dir.join("config/app.toml"), APP_TOML, &ctx, force)?;
    write_file(&dir.join("config/database.toml"), DATABASE_TOML, force)?;
    write_file(&dir.join("db/.gitkeep"), "", force)?;
    write_rendered(&dir.join("src/main.rs"), MAIN_RS, &ctx, force)?;
    write_file(&dir.join("src/controllers.rs"), CONTROLLERS_RS, force)?;

    if !api_only {
        write_file(&dir.join("templates/home.html"), HOME_HTML, force)?;
    }

    println!(
        "\n{}\n  cd {name}\n  cargo run -- migrate   {}\n  cargo run              {}\n",
        style("Done. Next:").bold(),
        style("# create the database schema").dim(),
        style(if api_only {
            "# serve the JSON API"
        } else {
            "# serve on http://127.0.0.1:8000"
        })
        .dim(),
    );
    Ok(())
}

const CARGO_TOML: &str = r#"[package]
name = "{{ crate_name }}"
version = "0.1.0"
edition = "2024"

[dependencies]
rustali = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde = { version = "1", features = ["derive"] }
{% if not api %}askama = "0.16"
{% endif %}
[dev-dependencies]
rustali = { version = "0.1", features = ["test"] }
"#;

const GITIGNORE: &str = "/target\n.env\n*.db\n*.db-shm\n*.db-wal\n";

const ENV: &str = "APP_ENV=development\n# APP__APP__KEY=set-a-32+-char-secret-in-production\n";

const APP_TOML: &str = r#"name = "{{ name }}"
env = "development"
debug = true
key = "dev-only-insecure-key-change-me-0123456789"
"#;

const DATABASE_TOML: &str = "url = \"sqlite://db/app.db?mode=rwc\"\n";

const MAIN_RS: &str = r#"mod controllers;

use rustali::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    App::builder()
        .provider(DatabaseProvider)
        .controllers()
        .run()
        .await
}
"#;

const CONTROLLERS_RS: &str = r#"use rustali::prelude::*;
use serde::Serialize;

pub struct HomeController;

#[derive(Serialize)]
struct Ping {
    ok: bool,
}

#[controller("/")]
impl HomeController {
    #[get("/", name = "home")]
    async fn home() -> &'static str {
        "Welcome to Rustali!"
    }

    #[get("/api/ping", name = "ping")]
    async fn ping() -> Json<Ping> {
        Json(Ping { ok: true })
    }
}
"#;

const HOME_HTML: &str = r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><title>Rustali</title></head>
<body><h1>Welcome to Rustali</h1></body>
</html>
"#;
