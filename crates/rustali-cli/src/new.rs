//! `rustali new <name>` — scaffold a fresh application from a preset + optional packages.
#![allow(clippy::needless_raw_string_hashes)]

use std::path::Path;

use anyhow::{Result, bail};
use console::style;
use dialoguer::{MultiSelect, Select, theme::ColorfulTheme};
use minijinja::context;

use crate::generate::{write_file, write_rendered};

/// Project preset: the starting shape of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Preset {
    /// JSON API with Swagger (openapi) wired in; no views.
    Api,
    /// Server-rendered site: views, sessions and auth out of the box.
    Fullstack,
    /// Bare core: ORM + one controller, nothing else.
    Minimal,
}

impl Preset {
    const ALL: [Self; 3] = [Self::Api, Self::Fullstack, Self::Minimal];

    const fn label(self) -> &'static str {
        match self {
            Self::Api => "api        — JSON API + Swagger",
            Self::Fullstack => "fullstack  — SSR views + sessions + auth",
            Self::Minimal => "minimal    — bare core",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "api" => Some(Self::Api),
            "fullstack" | "full" => Some(Self::Fullstack),
            "minimal" | "min" => Some(Self::Minimal),
            _ => None,
        }
    }
}

/// An optional package the user can add on top of the preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pkg {
    Cache,
    Mail,
    Queue,
    Auth,
    Openapi,
}

impl Pkg {
    const ALL: [Self; 5] = [
        Self::Cache,
        Self::Mail,
        Self::Queue,
        Self::Auth,
        Self::Openapi,
    ];

    const fn key(self) -> &'static str {
        match self {
            Self::Cache => "cache",
            Self::Mail => "mail",
            Self::Queue => "queue",
            Self::Auth => "auth",
            Self::Openapi => "openapi",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Cache => "cache    — in-memory/Redis cache",
            Self::Mail => "mail     — Mailable + transports",
            Self::Queue => "queue    — background jobs",
            Self::Auth => "auth     — users, login, sessions (+ migration)",
            Self::Openapi => "openapi  — OpenAPI spec + Swagger UI",
        }
    }
}

/// The computed scaffold: everything that depends on the preset + package choices.
#[derive(Debug, Default)]
struct ScaffoldPlan {
    /// Cargo features for the `rustali` dependency.
    features: Vec<&'static str>,
    /// `mod` declarations for the top of `main.rs`.
    modules: Vec<&'static str>,
    /// Statements before `App::builder()` (e.g. the queue worker).
    preamble: Vec<&'static str>,
    /// Builder method calls, in order, each like `.provider(CacheProvider)`.
    builder: Vec<String>,
    /// Extra `.env` lines contributed by the chosen packages.
    env_lines: Vec<&'static str>,
    /// Extra source files to write: (relative path, contents).
    files: Vec<(&'static str, &'static str)>,
    /// Whether to emit `templates/home.html` + the askama dependency.
    views: bool,
}

/// Compute the scaffold for a preset plus the chosen optional packages.
///
/// Pure and deterministic — the unit tests exercise it directly, since the generated project
/// can't be compiled here (`rustali` is unpublished).
fn plan(preset: Preset, pkgs: &[Pkg]) -> ScaffoldPlan {
    let mut p = ScaffoldPlan::default();
    // Every app uses the ORM + a database.
    p.builder.push(".provider(DatabaseProvider)".to_owned());

    let has = |pkg: Pkg| pkgs.contains(&pkg);
    // Auth is a package, but the fullstack preset turns it on by default.
    let auth = has(Pkg::Auth) || preset == Preset::Fullstack;
    let openapi = has(Pkg::Openapi) || preset == Preset::Api;
    p.views = preset == Preset::Fullstack;

    // --- auth (+ sessions, models, migration) ---
    if auth {
        p.features.push("auth");
        p.modules.push("mod models;");
        p.modules.push("mod migrations;");
        p.builder.push(".provider(SessionProvider)".to_owned());
        p.builder
            .push(".provider(AuthProvider::new(models::Users))".to_owned());
        p.files.push(("src/models.rs", MODELS_RS));
        p.files.push(("src/migrations.rs", MIGRATIONS_RS));
        p.files.push((
            "src/migrations/m20260101_000001_create_users.rs",
            CREATE_USERS_RS,
        ));
    }

    // --- simple providers ---
    if has(Pkg::Cache) {
        p.features.push("cache");
        p.builder.push(".provider(CacheProvider)".to_owned());
        p.env_lines.push("CACHE_DRIVER=memory");
    }
    if has(Pkg::Mail) {
        p.features.push("mail");
        p.builder.push(".provider(MailProvider)".to_owned());
        p.env_lines.push("MAIL_TRANSPORT=log");
        p.env_lines.push("MAIL_FROM=no-reply@example.com");
    }
    if has(Pkg::Queue) {
        p.features.push("queue");
        p.preamble.push("let worker = Worker::builder().build();");
        p.builder
            .push(".provider(QueueProvider::new(worker))".to_owned());
        p.env_lines.push("QUEUE_CAPACITY=1024");
    }

    // --- migrations call (after providers, before controllers) ---
    if auth {
        p.builder
            .push(".migrations::<migrations::Migrator>()".to_owned());
        p.env_lines.push("SESSION_CSRF=true");
    }

    // --- controllers (always) then the openapi router ---
    p.builder.push(".controllers()".to_owned());
    if openapi {
        p.features.push("openapi");
        p.builder.push(".routes(openapi_router())".to_owned());
        p.env_lines.push("OPENAPI_TITLE=API");
    }

    // --- session layers last (needs the context) ---
    if auth {
        p.builder.push(".configure(session_layers)".to_owned());
    }

    p.builder.push(".run()".to_owned());
    p.builder.push(".await".to_owned());
    p
}

/// Scaffold a new project. `args` is everything after `new`.
pub(crate) fn run(args: &[String]) -> Result<()> {
    let force = args.iter().any(|a| a == "--force");
    let name = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!("usage: rustali new <name> [--preset api|fullstack|minimal]")
        })?;

    let preset = resolve_preset(args)?;
    let pkgs = resolve_packages(args, preset)?;

    let dir = Path::new(&name);
    if dir.exists() && !force {
        bail!("directory `{name}` already exists (use --force to write into it)");
    }

    let crate_name = name.replace(['-', ' '], "_");
    let scaffold = plan(preset, &pkgs);

    println!("{} {name}", style("Creating").green().bold());

    // Cargo.toml — the rustali feature list (empty = default features).
    let features = render_feature_list(&scaffold.features);
    let cargo_ctx =
        context! { crate_name => crate_name, features => features, views => scaffold.views };
    write_rendered(&dir.join("Cargo.toml"), CARGO_TOML, &cargo_ctx, force)?;

    // main.rs — assembled from the plan.
    let main_ctx = context! {
        modules => scaffold.modules.join("\n"),
        preamble => scaffold.preamble.join("\n    "),
        builder => render_builder(&scaffold.builder),
    };
    write_rendered(&dir.join("src/main.rs"), MAIN_RS, &main_ctx, force)?;

    // Fixed files.
    write_file(&dir.join(".gitignore"), GITIGNORE, force)?;
    // The single .env: base settings plus whatever the packages contributed.
    let env_ctx = context! { name => name, extra => scaffold.env_lines.join("\n") };
    write_rendered(&dir.join(".env"), ENV, &env_ctx, force)?;
    write_file(&dir.join("db/.gitkeep"), "", force)?;
    write_file(&dir.join("src/controllers.rs"), CONTROLLERS_RS, force)?;

    if scaffold.views {
        write_file(&dir.join("templates/home.html"), HOME_HTML, force)?;
    }
    for (path, contents) in &scaffold.files {
        write_file(&dir.join(path), contents, force)?;
    }

    print_next_steps(&name, preset);
    Ok(())
}

/// Pick the preset from `--preset <name>` / `--api`, else prompt interactively.
fn resolve_preset(args: &[String]) -> Result<Preset> {
    if args.iter().any(|a| a == "--api") {
        return Ok(Preset::Api);
    }
    if let Some(value) = flag_value(args, "--preset") {
        return Preset::parse(&value)
            .ok_or_else(|| anyhow::anyhow!("unknown preset `{value}` (api|fullstack|minimal)"));
    }
    let labels: Vec<&str> = Preset::ALL.iter().map(|p| p.label()).collect();
    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Project type")
        .items(&labels)
        .default(0)
        .interact()?;
    Ok(Preset::ALL[choice])
}

/// Pick optional packages from `--with a,b` / repeated flags, else prompt (multi-select).
fn resolve_packages(args: &[String], preset: Preset) -> Result<Vec<Pkg>> {
    // Non-interactive: `--with cache,mail` or `--no-prompt` for just the preset.
    if let Some(value) = flag_value(args, "--with") {
        let mut out = Vec::new();
        for key in value.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let pkg = Pkg::ALL
                .iter()
                .find(|p| p.key() == key)
                .ok_or_else(|| anyhow::anyhow!("unknown package `{key}`"))?;
            out.push(*pkg);
        }
        return Ok(out);
    }
    if args.iter().any(|a| a == "--no-prompt" || a == "--api") {
        return Ok(Vec::new());
    }

    let labels: Vec<&str> = Pkg::ALL.iter().map(|p| p.label()).collect();
    // Pre-check the packages the preset already implies, as a hint.
    let defaults: Vec<bool> = Pkg::ALL
        .iter()
        .map(|p| match p {
            Pkg::Auth => preset == Preset::Fullstack,
            Pkg::Openapi => preset == Preset::Api,
            _ => false,
        })
        .collect();
    let chosen = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Extra packages (space to toggle, enter to confirm)")
        .items(&labels)
        .defaults(&defaults)
        .interact()?;
    Ok(chosen.into_iter().map(|i| Pkg::ALL[i]).collect())
}

/// The value following `flag` in `args`, if present (`--preset api`).
fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// Render the Cargo `features = [...]` list, or empty string for default features.
fn render_feature_list(features: &[&str]) -> String {
    if features.is_empty() {
        return String::new();
    }
    let mut unique: Vec<&str> = Vec::new();
    for f in features {
        if !unique.contains(f) {
            unique.push(f);
        }
    }
    let joined = unique
        .iter()
        .map(|f| format!("\"{f}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(", features = [{joined}]")
}

/// Render the builder method chain, indented under `App::builder()`.
fn render_builder(lines: &[String]) -> String {
    lines
        .iter()
        .map(|l| format!("        {l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn print_next_steps(name: &str, preset: Preset) {
    let serve_note = match preset {
        Preset::Api => "# serve the JSON API — Swagger at /swagger",
        _ => "# serve on http://127.0.0.1:8000",
    };
    println!(
        "\n{}\n  cd {name}\n  cargo run -- migrate   {}\n  cargo run              {}\n",
        style("Done. Next:").bold(),
        style("# create the database schema").dim(),
        style(serve_note).dim(),
    );
}

// --- templates ---------------------------------------------------------------

const CARGO_TOML: &str = r#"[package]
name = "{{ crate_name }}"
version = "0.1.0"
edition = "2024"

[dependencies]
rustali = { version = "0.1"{{ features }} }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde = { version = "1", features = ["derive"] }
{% if views %}askama = "0.16"
{% endif %}
[dev-dependencies]
rustali = { version = "0.1", features = ["test"] }
"#;

const MAIN_RS: &str = r#"{{ modules }}{% if modules %}

{% endif %}use rustali::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    {% if preamble %}{{ preamble }}
    {% endif %}App::builder()
{{ builder }}
}
"#;

const GITIGNORE: &str = "/target\n*.db\n*.db-shm\n*.db-wal\n";

/// The single configuration file. Base settings plus any package lines, then a note that
/// real secrets come from the environment in production.
const ENV: &str = r#"APP_NAME={{ name }}
APP_ENV=development
APP_DEBUG=true
# Set a real secret (>= 16 chars) from the environment in production.
APP_KEY=dev-only-insecure-key-change-me-0123456789

DATABASE_URL=sqlite://db/app.db?mode=rwc
{% if extra %}
{{ extra }}
{% endif %}"#;

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

const MODELS_RS: &str = r#"//! The `User` entity and its store, for authentication.

use rustali::{
    orm::chrono::{DateTime, Utc},
    prelude::*,
};

#[entity]
#[table("users")]
pub struct User {
    pub id: i64,
    pub name: String,
    #[column(unique)]
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    #[must_use]
    pub fn new(name: &str, email: &str, password_hash: String) -> Self {
        Self {
            id: 0,
            name: name.to_owned(),
            email: email.to_owned(),
            password_hash,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Authenticatable for User {
    fn auth_id(&self) -> String {
        self.id.to_string()
    }

    fn password_hash(&self) -> &str {
        &self.password_hash
    }
}

/// The user store that authentication loads users through.
pub struct Users;

impl UserStore for Users {
    type User = User;

    fn find_by_id<'a>(&'a self, ctx: &'a AppContext, id: &'a str) -> StoreFuture<'a, Option<User>> {
        Box::pin(async move {
            let Ok(id) = id.parse::<i64>() else {
                return Ok(None);
            };
            User::find(&*ctx.db()?, id).await
        })
    }

    fn find_by_login<'a>(
        &'a self,
        ctx: &'a AppContext,
        email: &'a str,
    ) -> StoreFuture<'a, Option<User>> {
        Box::pin(async move {
            User::query()
                .filter(User::EMAIL.eq(email))
                .fetch_one(&*ctx.db()?)
                .await
        })
    }
}
"#;

const MIGRATIONS_RS: &str = r#"//! Database migrations.

use rustali::migrate::prelude::*;

mod m20260101_000001_create_users;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260101_000001_create_users::Migration)]
    }
}
"#;

const CREATE_USERS_RS: &str = r#"//! Create the `users` table.

#![allow(elided_lifetimes_in_paths)]

use rustali::migrate::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Users::Name).string_len(50).not_null())
                    .col(
                        ColumnDef::new(Users::Email)
                            .string_len(255)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Users::PasswordHash).string().not_null())
                    .col(
                        ColumnDef::new(Users::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Users::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Name,
    Email,
    PasswordHash,
    CreatedAt,
    UpdatedAt,
}
"#;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn builder_text(plan: &ScaffoldPlan) -> String {
        plan.builder.join("\n")
    }

    #[test]
    fn api_preset_adds_openapi() {
        let p = plan(Preset::Api, &[]);
        assert!(p.features.contains(&"openapi"));
        assert!(builder_text(&p).contains(".routes(openapi_router())"));
        assert!(builder_text(&p).contains(".controllers()"));
        assert!(!p.views, "api has no views");
    }

    #[test]
    fn fullstack_preset_has_auth_and_views() {
        let p = plan(Preset::Fullstack, &[]);
        assert!(p.features.contains(&"auth"));
        assert!(p.views);
        assert!(builder_text(&p).contains(".provider(SessionProvider)"));
        assert!(builder_text(&p).contains(".provider(AuthProvider::new(models::Users))"));
        assert!(builder_text(&p).contains(".migrations::<migrations::Migrator>()"));
        assert!(builder_text(&p).contains(".configure(session_layers)"));
        assert!(p.files.iter().any(|(path, _)| *path == "src/models.rs"));
        assert!(
            p.files
                .iter()
                .any(|(path, _)| path.contains("create_users"))
        );
    }

    #[test]
    fn minimal_preset_is_bare() {
        let p = plan(Preset::Minimal, &[]);
        assert_eq!(p.features, Vec::<&str>::new(), "no extra features");
        assert!(!p.views);
        assert_eq!(p.files.len(), 0);
        assert_eq!(builder_text(&p).matches(".provider(").count(), 1); // only DatabaseProvider
    }

    #[test]
    fn minimal_plus_cache_and_mail() {
        let p = plan(Preset::Minimal, &[Pkg::Cache, Pkg::Mail]);
        assert!(p.features.contains(&"cache"));
        assert!(p.features.contains(&"mail"));
        assert!(builder_text(&p).contains(".provider(CacheProvider)"));
        assert!(builder_text(&p).contains(".provider(MailProvider)"));
        assert!(p.env_lines.contains(&"CACHE_DRIVER=memory"));
        assert!(p.env_lines.contains(&"MAIL_TRANSPORT=log"));
    }

    #[test]
    fn api_plus_auth_wires_everything() {
        let p = plan(Preset::Api, &[Pkg::Auth]);
        assert!(p.features.contains(&"auth"));
        assert!(p.features.contains(&"openapi"));
        assert!(builder_text(&p).contains(".provider(AuthProvider::new(models::Users))"));
        assert!(builder_text(&p).contains(".migrations::<migrations::Migrator>()"));
        assert!(builder_text(&p).contains(".routes(openapi_router())"));
    }

    #[test]
    fn queue_adds_worker_preamble() {
        let p = plan(Preset::Minimal, &[Pkg::Queue]);
        assert!(p.preamble.iter().any(|l| l.contains("Worker::builder()")));
        assert!(builder_text(&p).contains(".provider(QueueProvider::new(worker))"));
    }

    #[test]
    fn builder_order_is_correct() {
        let p = plan(Preset::Fullstack, &[]);
        let text = builder_text(&p);
        let db = text.find("DatabaseProvider").unwrap();
        let controllers = text.find(".controllers()").unwrap();
        let migrations = text.find(".migrations::").unwrap();
        let configure = text.find(".configure(").unwrap();
        let run = text.find(".run()").unwrap();
        assert!(db < migrations, "providers before migrations");
        assert!(migrations < controllers, "migrations before controllers");
        assert!(controllers < configure, "controllers before configure");
        assert!(configure < run, "configure before run");
    }

    #[test]
    fn feature_list_rendering() {
        assert_eq!(render_feature_list(&[]), "");
        assert_eq!(
            render_feature_list(&["auth", "openapi"]),
            ", features = [\"auth\", \"openapi\"]"
        );
        // dedups
        assert_eq!(
            render_feature_list(&["cache", "cache"]),
            ", features = [\"cache\"]"
        );
    }
}
