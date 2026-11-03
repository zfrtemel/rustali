//! `rustali new <name>` — scaffold a fresh application from a preset + optional packages.
#![allow(clippy::needless_raw_string_hashes)]

use std::fmt::Write as _;
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

/// The database driver the app connects with. SQLite needs nothing installed; the other two
/// talk to a running server. Each one maps to a Cargo feature on `rustali` and a `DATABASE_URL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Db {
    /// File-backed SQLite — the default, needs nothing installed.
    #[default]
    Sqlite,
    /// PostgreSQL server.
    Postgres,
    /// `MySQL` / `MariaDB` server.
    Mysql,
}

impl Db {
    const ALL: [Self; 3] = [Self::Sqlite, Self::Postgres, Self::Mysql];

    const fn label(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite     — file, nothing to install (default)",
            Self::Postgres => "postgres   — PostgreSQL server",
            Self::Mysql => "mysql      — MySQL / MariaDB server",
        }
    }

    /// The Cargo feature that pulls in this driver.
    const fn feature(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite",
            Self::Postgres => "postgres",
            Self::Mysql => "mysql",
        }
    }

    /// A ready-to-edit `DATABASE_URL` for this driver.
    const fn url(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite://db/app.db?mode=rwc",
            Self::Postgres => "postgres://postgres:postgres@localhost:5432/app",
            Self::Mysql => "mysql://root:root@localhost:3306/app",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "sqlite" => Some(Self::Sqlite),
            "postgres" | "postgresql" | "pg" => Some(Self::Postgres),
            "mysql" | "mariadb" => Some(Self::Mysql),
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
    /// Whether auth is active (controls the auth controller + `mod auth;`).
    auth: bool,
    /// The chosen database driver (controls the Cargo driver feature + `DATABASE_URL`).
    db: Db,
}

/// Compute the scaffold for a preset plus the chosen optional packages.
///
/// Pure and deterministic — the unit tests exercise it directly, since the generated project
/// can't be compiled here (`rustali` is unpublished).
fn plan(preset: Preset, pkgs: &[Pkg], db: Db) -> ScaffoldPlan {
    let mut p = ScaffoldPlan::default();
    // Every app uses the ORM + a database.
    p.builder.push(".provider(DatabaseProvider)".to_owned());
    p.db = db;
    // Every app has a controllers module (written as a folder in `run`); declare it so the
    // binary actually compiles `src/controllers/` and its #[controller]s auto-register.
    p.modules.push("mod controllers;");

    let has = |pkg: Pkg| pkgs.contains(&pkg);
    // Auth is a package, but the fullstack preset turns it on by default.
    let auth = has(Pkg::Auth) || preset == Preset::Fullstack;
    let openapi = has(Pkg::Openapi) || preset == Preset::Api;
    p.views = preset == Preset::Fullstack;
    p.auth = auth;

    // --- auth (+ sessions, models, migration, a working register/login flow) ---
    if auth {
        p.features.push("auth");
        p.modules.push("mod models;");
        p.modules.push("mod migrations;");
        p.builder.push(".provider(SessionProvider)".to_owned());
        p.builder
            .push(".provider(AuthProvider::new(models::Users))".to_owned());
        p.files.push(("src/models/user.rs", MODELS_RS));
        p.files.push(("src/models/mod.rs", MODELS_MOD));
        p.files.push(("src/migrations/mod.rs", MIGRATIONS_RS));
        p.files.push((
            "src/migrations/m20260101_000001_create_users.rs",
            CREATE_USERS_RS,
        ));
        // A ready-to-run auth flow: SSR (views) when the preset has views, otherwise a
        // JSON API that returns a JWT bearer token.
        if p.views {
            p.files
                .push(("src/controllers/auth.rs", AUTH_CONTROLLER_SSR));
            p.files.push(("templates/layout.html", AUTH_LAYOUT_HTML));
            p.files
                .push(("templates/register.html", AUTH_REGISTER_HTML));
            p.files.push(("templates/login.html", AUTH_LOGIN_HTML));
            p.files
                .push(("templates/dashboard.html", AUTH_DASHBOARD_HTML));
            p.files.push(("askama.toml", ASKAMA_TOML));
        } else {
            p.files
                .push(("src/controllers/auth.rs", AUTH_CONTROLLER_API));
        }
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

/// The `rustali` dependency line's feature config for the chosen driver.
///
/// SQLite is a default feature, so the common case just appends the package features to the
/// defaults. Any other driver means turning the defaults off (`default-features = false`) and
/// listing every feature explicitly — otherwise SQLite would compile in alongside the real
/// driver. Returns `(default_features, features)`: the feature list to render, which already
/// includes the driver and (for non-sqlite) the core features the defaults would have given.
fn driver_features(db: Db, pkg_features: &[&'static str]) -> (bool, Vec<&'static str>) {
    if db == Db::Sqlite {
        // Defaults already include sqlite; just add the package features on top.
        return (true, pkg_features.to_vec());
    }
    // Non-sqlite: reproduce the default feature set, swap sqlite for the chosen driver, then
    // add the package features (deduped later by `render_feature_list`).
    let mut out = vec![
        "macros",
        "orm",
        db.feature(),
        "views",
        "sessions",
        "validation",
        "auth",
        "cache",
        "mail",
        "queue",
    ];
    out.extend_from_slice(pkg_features);
    (false, out)
}

/// Scaffold a new project. `args` is everything after `new`.
pub(crate) fn run(args: &[String]) -> Result<()> {
    let force = args.iter().any(|a| a == "--force");
    let name = project_name(args).ok_or_else(|| {
        anyhow::anyhow!(
            "usage: rustali new <name> [--preset api|fullstack|minimal] [--database sqlite|postgres|mysql]"
        )
    })?;

    let preset = resolve_preset(args)?;
    let pkgs = resolve_packages(args, preset)?;
    let db = resolve_database(args)?;

    let dir = Path::new(&name);
    if dir.exists() && !force {
        bail!("directory `{name}` already exists (use --force to write into it)");
    }

    let crate_name = name.replace(['-', ' '], "_");
    let scaffold = plan(preset, &pkgs, db);

    println!("{} {name}", style("Creating").green().bold());

    // Cargo.toml — the rustali dependency line. SQLite rides the default features; any other
    // driver turns the defaults off and lists everything (including the driver) explicitly.
    let (default_features, feature_list) = driver_features(scaffold.db, &scaffold.features);
    let features = render_feature_list(default_features, &feature_list);
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
    // The single .env: base settings plus whatever the packages contributed. The chosen driver's
    // URL is active; the other two are shown commented, each with the one-line Cargo change.
    let env_ctx = context! {
        name => name,
        database_url => scaffold.db.url(),
        database_hints => database_env_hints(scaffold.db),
        extra => scaffold.env_lines.join("\n"),
    };
    write_rendered(&dir.join(".env"), ENV, &env_ctx, force)?;
    write_file(&dir.join("db/.gitkeep"), "", force)?;
    // Controllers live in a folder so `make:controller` can add siblings without colliding
    // with a flat `controllers.rs` (Rust forbids both for the same module).
    let controllers_mod = if scaffold.auth {
        CONTROLLERS_MOD_WITH_AUTH
    } else {
        CONTROLLERS_MOD
    };
    write_file(&dir.join("src/controllers/mod.rs"), controllers_mod, force)?;
    write_file(&dir.join("src/controllers/home.rs"), CONTROLLERS_RS, force)?;

    if scaffold.views {
        write_file(&dir.join("templates/home.html"), HOME_HTML, force)?;
    }
    for (path, contents) in &scaffold.files {
        write_file(&dir.join(path), contents, force)?;
    }

    print_next_steps(&name, preset, scaffold.db);
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

/// Pick the database driver from `--database <name>`, else prompt interactively (unless a
/// non-interactive flag is set, in which case SQLite — the zero-install default — is used).
fn resolve_database(args: &[String]) -> Result<Db> {
    if let Some(value) = flag_value(args, "--database").or_else(|| flag_value(args, "--db")) {
        return Db::parse(&value)
            .ok_or_else(|| anyhow::anyhow!("unknown database `{value}` (sqlite|postgres|mysql)"));
    }
    // Fully non-interactive runs keep the zero-install default rather than blocking on a prompt.
    if args.iter().any(|a| a == "--no-prompt" || a == "--api") {
        return Ok(Db::default());
    }

    let labels: Vec<&str> = Db::ALL.iter().map(|d| d.label()).collect();
    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Database")
        .items(&labels)
        .default(0)
        .interact()?;
    Ok(Db::ALL[choice])
}

/// Flags that take a value (`--preset api`); their following arg is consumed, not the name.
const VALUE_FLAGS: [&str; 4] = ["--preset", "--with", "--database", "--db"];

/// The project name: the first positional arg, skipping flags and the values they consume.
/// This keeps `rustali new --database postgres blog` from mistaking `postgres` for the name.
fn project_name(args: &[String]) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if VALUE_FLAGS.contains(&arg.as_str()) {
            i += 2; // skip the flag and its value
            continue;
        }
        if arg.starts_with('-') {
            i += 1; // a boolean flag like --force / --no-prompt
            continue;
        }
        return Some(arg.clone());
    }
    None
}

/// The value following `flag` in `args`, if present (`--preset api`).
fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// Commented `.env` lines for the drivers *not* chosen, so switching later is a one-line edit.
/// Each names the Cargo feature it needs, since a driver is a feature on `rustali`.
fn database_env_hints(active: Db) -> String {
    let mut out = String::new();
    for db in Db::ALL {
        if db == active {
            continue;
        }
        let name = match db {
            Db::Sqlite => "SQLite",
            Db::Postgres => "PostgreSQL",
            Db::Mysql => "MySQL / MariaDB",
        };
        let _ = write!(
            out,
            "# {name} — set `features = [\"{feat}\"]` on rustali in Cargo.toml, then:\n# DATABASE_URL={url}\n",
            feat = db.feature(),
            url = db.url(),
        );
    }
    // Trim the trailing newline; the template adds its own spacing.
    out.truncate(out.trim_end().len());
    out
}

/// Render the extra text inside the `rustali = { version = "0.1"<here> }` dependency line.
///
/// With default features and no extras this is empty. `default_features = false` (a non-sqlite
/// driver) prepends `default-features = false`, and any features are deduped and listed.
fn render_feature_list(default_features: bool, features: &[&str]) -> String {
    let mut unique: Vec<&str> = Vec::new();
    for f in features {
        if !unique.contains(f) {
            unique.push(f);
        }
    }

    let mut out = String::new();
    if !default_features {
        out.push_str(", default-features = false");
    }
    if !unique.is_empty() {
        let joined = unique
            .iter()
            .map(|f| format!("\"{f}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, ", features = [{joined}]");
    }
    out
}

/// Render the builder method chain, indented under `App::builder()`.
fn render_builder(lines: &[String]) -> String {
    lines
        .iter()
        .map(|l| format!("        {l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn print_next_steps(name: &str, preset: Preset, db: Db) {
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
    if db != Db::Sqlite {
        let server = if db == Db::Postgres {
            "Postgres"
        } else {
            "MySQL"
        };
        println!(
            "{} {server} must be running and DATABASE_URL in .env must point at it before `migrate`.\n",
            style("note:").yellow().bold(),
        );
    }
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

# Switch databases by editing DATABASE_URL (and the matching feature in Cargo.toml).
DATABASE_URL={{ database_url }}
{{ database_hints }}
{% if extra %}
{{ extra }}
{% endif %}"#;

/// The controllers aggregator: links each controller sub-module so its `#[controller]`
/// auto-registers. No `pub use` needed — controllers are collected via inventory, not by name.
const CONTROLLERS_MOD: &str = "mod home;\n";

/// The controllers aggregator when auth is scaffolded: also links `auth.rs`.
const CONTROLLERS_MOD_WITH_AUTH: &str = "mod home;\nmod auth;\n";

/// The models aggregator: links and re-exports each entity so `models::User` resolves from
/// `main.rs` (auth wiring refers to `models::Users` by path).
const MODELS_MOD: &str = "mod user;\npub use user::*;\n";

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

/// `askama.toml` — tells Askama where templates live (compile-time embedding).
const ASKAMA_TOML: &str = "[general]\ndirs = [\"templates\"]\n";

/// Server-rendered auth: register / login / logout / dashboard with views, CSRF, flash and
/// validation. Adapted from the `auth-app` example (its `/` home action is dropped so it does
/// not clash with `HomeController`).
const AUTH_CONTROLLER_SSR: &str = r#"//! Register, login, logout and a protected dashboard.

use askama::Template;
use rustali::{http::StatusCode, prelude::*};
use serde::Deserialize;

use crate::models::User;

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterPage {
    app_name: String,
    csrf_field: String,
    old_name: String,
    old_email: String,
    name_error: Option<String>,
    email_error: Option<String>,
    password_error: Option<String>,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage {
    app_name: String,
    csrf_field: String,
    flash: Vec<rustali::session::Message>,
    error: Option<String>,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardPage {
    app_name: String,
    user_name: String,
    user_email: String,
    csrf_field: String,
}

#[derive(Deserialize, Validate)]
struct RegisterForm {
    #[rule(required, length(min = 2, max = 50))]
    name: String,
    #[rule(required, email)]
    email: String,
    #[rule(required, length(min = 8), confirmed)]
    password: String,
    #[serde(default)]
    password_confirmation: String,
    #[serde(default, rename = "_token")]
    _token: String,
}

#[derive(Deserialize)]
struct LoginForm {
    email: String,
    password: String,
    #[serde(default, rename = "_token")]
    _token: String,
}

pub struct AuthController;

#[controller("/")]
impl AuthController {
    #[get("/register", name = "register")]
    async fn register_form(ctx: Ctx, csrf: Csrf, flash: Flash) -> Result<View<RegisterPage>> {
        let errors = flash.errors().await;
        let page = RegisterPage {
            app_name: ctx.config().app.name.clone(),
            csrf_field: csrf.field().await,
            old_name: flash.old("name").await.unwrap_or_default(),
            old_email: flash.old("email").await.unwrap_or_default(),
            name_error: errors.first("name").map(str::to_owned),
            email_error: errors.first("email").map(str::to_owned),
            password_error: errors.first("password").map(str::to_owned),
        };
        flash.clear_old().await;
        Ok(View::new(page))
    }

    #[post("/register", name = "register.store")]
    async fn register(
        db: Db,
        flash: Flash,
        login: Login<User>,
        Validated(Form(input)): Validated<Form<RegisterForm>>,
    ) -> Result<Redirect> {
        if User::query()
            .filter(User::EMAIL.eq(&input.email))
            .exists(&db)
            .await?
        {
            flash.error("That email is already registered.").await;
            return Ok(Redirect::to("/register"));
        }
        let hash = password::hash(&input.password)?;
        let user = User::new(input.name.trim(), input.email.trim(), hash)
            .insert(&db)
            .await?;
        login.login(&user).await?;
        flash.success("Welcome!").await;
        Ok(Redirect::to("/dashboard"))
    }

    #[get("/login", name = "login")]
    async fn login_form(ctx: Ctx, csrf: Csrf, flash: Flash) -> Result<View<LoginPage>> {
        Ok(View::new(LoginPage {
            app_name: ctx.config().app.name.clone(),
            csrf_field: csrf.field().await,
            flash: flash.take().await,
            error: None,
        }))
    }

    #[post("/login", name = "login.store")]
    async fn login(
        ctx: Ctx,
        db: Db,
        csrf: Csrf,
        login: Login<User>,
        Form(input): Form<LoginForm>,
    ) -> Result<rustali::http::response::Response> {
        use rustali::http::response::IntoResponse;
        let user = User::query()
            .filter(User::EMAIL.eq(&input.email))
            .fetch_one(&db)
            .await?;
        let ok = match &user {
            Some(user) => login.attempt(user, &input.password).await?,
            None => false,
        };
        if ok {
            return Ok(Redirect::to("/dashboard").into_response());
        }
        Ok(View::new(LoginPage {
            app_name: ctx.config().app.name.clone(),
            csrf_field: csrf.field().await,
            flash: Vec::new(),
            error: Some("Those credentials do not match our records.".to_owned()),
        })
        .with_status(StatusCode::UNPROCESSABLE_ENTITY)
        .into_response())
    }

    #[post("/logout", name = "logout")]
    async fn logout(login: Login<User>, flash: Flash) -> Result<Redirect> {
        login.logout().await?;
        flash.success("Signed out.").await;
        Ok(Redirect::to("/login"))
    }

    #[get("/dashboard", name = "dashboard")]
    async fn dashboard(ctx: Ctx, csrf: Csrf, Auth(user): Auth<User>) -> Result<View<DashboardPage>> {
        Ok(View::new(DashboardPage {
            app_name: ctx.config().app.name.clone(),
            user_name: user.name,
            user_email: user.email,
            csrf_field: csrf.field().await,
        }))
    }
}
"#;

/// JSON auth for APIs: register / login return a JWT bearer token; `/api/me` reads it back.
///
/// JWT is stateless, so there is no server-side logout — a client logs out by discarding its
/// token.
const AUTH_CONTROLLER_API: &str = r#"//! JSON auth: register and login return a JWT bearer token.
//!
//! JWT is stateless, so there is no logout endpoint — a client logs out by discarding the
//! token. Send it back as `Authorization: Bearer <token>` to reach protected routes.

use rustali::{auth_kit::jwt, prelude::*};
use serde::{Deserialize, Serialize};

use crate::models::User;

/// How long an issued token stays valid (24 hours).
const TOKEN_TTL_SECS: i64 = 60 * 60 * 24;

#[derive(Deserialize, Validate)]
struct RegisterForm {
    #[rule(required, length(min = 2, max = 50))]
    name: String,
    #[rule(required, email)]
    email: String,
    #[rule(required, length(min = 8))]
    password: String,
}

#[derive(Deserialize)]
struct LoginForm {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct UserDto {
    id: i64,
    name: String,
    email: String,
}

#[derive(Serialize)]
struct TokenResponse {
    token: String,
    user: UserDto,
}

pub struct AuthController;

#[controller("/api")]
impl AuthController {
    #[post("/register", name = "api.register")]
    async fn register(
        ctx: Ctx,
        db: Db,
        Validated(Json(input)): Validated<Json<RegisterForm>>,
    ) -> Result<Json<TokenResponse>> {
        if User::query()
            .filter(User::EMAIL.eq(&input.email))
            .exists(&db)
            .await?
        {
            return Err(Error::bad_request("That email is already registered."));
        }
        let hash = password::hash(&input.password)?;
        let user = User::new(input.name.trim(), input.email.trim(), hash)
            .insert(&db)
            .await?;
        let token = jwt::issue(&ctx, &user.auth_id(), TOKEN_TTL_SECS)?;
        Ok(Json(TokenResponse {
            token,
            user: UserDto {
                id: user.id,
                name: user.name,
                email: user.email,
            },
        }))
    }

    #[post("/login", name = "api.login")]
    async fn login(
        ctx: Ctx,
        db: Db,
        Json(input): Json<LoginForm>,
    ) -> Result<Json<TokenResponse>> {
        let user = User::query()
            .filter(User::EMAIL.eq(&input.email))
            .fetch_one(&db)
            .await?;
        let ok = match &user {
            Some(u) => password::verify(&input.password, u.password_hash())?,
            None => false,
        };
        let Some(user) = user.filter(|_| ok) else {
            return Err(Error::Unauthorized);
        };
        let token = jwt::issue(&ctx, &user.auth_id(), TOKEN_TTL_SECS)?;
        Ok(Json(TokenResponse {
            token,
            user: UserDto {
                id: user.id,
                name: user.name,
                email: user.email,
            },
        }))
    }

    #[get("/me", name = "api.me")]
    async fn me(Auth(user): Auth<User>) -> Json<UserDto> {
        Json(UserDto {
            id: user.id,
            name: user.name,
            email: user.email,
        })
    }
}
"#;

const AUTH_LAYOUT_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{% block title %}{{ app_name }}{% endblock %}</title>
  <style>
    body { font-family: system-ui, sans-serif; max-width: 26rem; margin: 3rem auto; padding: 0 1rem; }
    .field { margin: .75rem 0; } label { display:block; font-weight:600; }
    input { width:100%; padding:.4rem; box-sizing:border-box; }
    .error { color:#c0392b; font-size:.9rem; } a { color:#b7410e; }
    .flash { background:#e6f4ea; padding:.5rem 1rem; border-radius:.4rem; }
  </style>
</head>
<body>
  <h1>{{ app_name }}</h1>
  {% block content %}{% endblock %}
</body>
</html>
"#;

const AUTH_REGISTER_HTML: &str = r#"{% extends "layout.html" %}
{% block title %}Register · {{ app_name }}{% endblock %}
{% block content %}
<h2>Create an account</h2>
<form method="post" action="/register">
  {{ csrf_field|safe }}
  <div class="field">
    <label for="name">Name</label>
    <input id="name" name="name" value="{{ old_name }}">
    {% if let Some(e) = name_error %}<p class="error">{{ e }}</p>{% endif %}
  </div>
  <div class="field">
    <label for="email">Email</label>
    <input id="email" name="email" type="email" value="{{ old_email }}">
    {% if let Some(e) = email_error %}<p class="error">{{ e }}</p>{% endif %}
  </div>
  <div class="field">
    <label for="password">Password</label>
    <input id="password" name="password" type="password">
    {% if let Some(e) = password_error %}<p class="error">{{ e }}</p>{% endif %}
  </div>
  <div class="field">
    <label for="password_confirmation">Confirm password</label>
    <input id="password_confirmation" name="password_confirmation" type="password">
  </div>
  <button type="submit">Register</button>
</form>
<p>Already have an account? <a href="/login">Sign in</a>.</p>
{% endblock %}
"#;

const AUTH_LOGIN_HTML: &str = r#"{% extends "layout.html" %}
{% block title %}Sign in · {{ app_name }}{% endblock %}
{% block content %}
<h2>Sign in</h2>
{% for message in flash %}<div class="flash">{{ message.text }}</div>{% endfor %}
{% if let Some(e) = error %}<p class="error">{{ e }}</p>{% endif %}
<form method="post" action="/login">
  {{ csrf_field|safe }}
  <div class="field">
    <label for="email">Email</label>
    <input id="email" name="email" type="email">
  </div>
  <div class="field">
    <label for="password">Password</label>
    <input id="password" name="password" type="password">
  </div>
  <button type="submit">Sign in</button>
</form>
<p>Need an account? <a href="/register">Register</a>.</p>
{% endblock %}
"#;

const AUTH_DASHBOARD_HTML: &str = r#"{% extends "layout.html" %}
{% block title %}Dashboard · {{ app_name }}{% endblock %}
{% block content %}
<h2>Welcome, {{ user_name }}</h2>
<p>You are signed in as {{ user_email }}.</p>
<form method="post" action="/logout">
  {{ csrf_field|safe }}
  <button type="submit">Sign out</button>
</form>
{% endblock %}
"#;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn builder_text(plan: &ScaffoldPlan) -> String {
        plan.builder.join("\n")
    }

    /// Most tests don't care about the driver; default to SQLite.
    fn scaffold(preset: Preset, pkgs: &[Pkg]) -> ScaffoldPlan {
        plan(preset, pkgs, Db::Sqlite)
    }

    #[test]
    fn api_preset_adds_openapi() {
        let p = scaffold(Preset::Api, &[]);
        assert!(p.features.contains(&"openapi"));
        assert!(builder_text(&p).contains(".routes(openapi_router())"));
        assert!(builder_text(&p).contains(".controllers()"));
        assert!(!p.views, "api has no views");
    }

    #[test]
    fn fullstack_preset_has_auth_and_views() {
        let p = scaffold(Preset::Fullstack, &[]);
        assert!(p.features.contains(&"auth"));
        assert!(p.views);
        assert!(builder_text(&p).contains(".provider(SessionProvider)"));
        assert!(builder_text(&p).contains(".provider(AuthProvider::new(models::Users))"));
        assert!(builder_text(&p).contains(".migrations::<migrations::Migrator>()"));
        assert!(builder_text(&p).contains(".configure(session_layers)"));
        assert!(
            p.files
                .iter()
                .any(|(path, _)| *path == "src/models/user.rs")
        );
        assert!(p.files.iter().any(|(path, _)| *path == "src/models/mod.rs"));
        assert!(
            p.files
                .iter()
                .any(|(path, _)| *path == "src/migrations/mod.rs")
        );
        assert!(
            p.files
                .iter()
                .any(|(path, _)| path.contains("create_users"))
        );
    }

    #[test]
    fn fullstack_auth_writes_ssr_controller_and_templates() {
        let p = scaffold(Preset::Fullstack, &[]);
        assert!(p.auth);
        let (_, body) = p
            .files
            .iter()
            .find(|(path, _)| *path == "src/controllers/auth.rs")
            .expect("auth controller");
        assert!(body.contains("View<"), "SSR controller returns View");
        assert!(!body.contains("jwt::issue"), "SSR controller has no JWT");
        for t in [
            "templates/layout.html",
            "templates/register.html",
            "templates/login.html",
            "templates/dashboard.html",
            "askama.toml",
        ] {
            assert!(p.files.iter().any(|(path, _)| *path == t), "missing {t}");
        }
    }

    #[test]
    fn api_auth_writes_jwt_controller_no_templates() {
        let p = scaffold(Preset::Api, &[Pkg::Auth]);
        assert!(p.auth);
        let (_, body) = p
            .files
            .iter()
            .find(|(path, _)| *path == "src/controllers/auth.rs")
            .expect("auth controller");
        assert!(body.contains("jwt::issue"), "API controller issues a JWT");
        assert!(body.contains("Json"));
        assert!(!body.contains("View<"), "API controller has no views");
        assert!(
            !p.files
                .iter()
                .any(|(path, _)| path.starts_with("templates/")),
            "API preset writes no templates"
        );
    }

    #[test]
    fn minimal_auth_uses_api_controller() {
        let p = scaffold(Preset::Minimal, &[Pkg::Auth]);
        let (_, body) = p
            .files
            .iter()
            .find(|(path, _)| *path == "src/controllers/auth.rs")
            .expect("auth controller");
        assert!(
            body.contains("jwt::issue"),
            "no-views auth → JWT API controller"
        );
    }

    #[test]
    fn every_preset_declares_controllers_module() {
        for preset in Preset::ALL {
            let p = scaffold(preset, &[]);
            assert!(
                p.modules.contains(&"mod controllers;"),
                "{preset:?} must declare `mod controllers;` so controllers compile"
            );
        }
    }

    #[test]
    fn minimal_preset_is_bare() {
        let p = scaffold(Preset::Minimal, &[]);
        assert_eq!(p.features, Vec::<&str>::new(), "no extra features");
        assert!(!p.views);
        assert_eq!(p.files.len(), 0);
        assert_eq!(builder_text(&p).matches(".provider(").count(), 1); // only DatabaseProvider
    }

    #[test]
    fn minimal_plus_cache_and_mail() {
        let p = scaffold(Preset::Minimal, &[Pkg::Cache, Pkg::Mail]);
        assert!(p.features.contains(&"cache"));
        assert!(p.features.contains(&"mail"));
        assert!(builder_text(&p).contains(".provider(CacheProvider)"));
        assert!(builder_text(&p).contains(".provider(MailProvider)"));
        assert!(p.env_lines.contains(&"CACHE_DRIVER=memory"));
        assert!(p.env_lines.contains(&"MAIL_TRANSPORT=log"));
    }

    #[test]
    fn api_plus_auth_wires_everything() {
        let p = scaffold(Preset::Api, &[Pkg::Auth]);
        assert!(p.features.contains(&"auth"));
        assert!(p.features.contains(&"openapi"));
        assert!(builder_text(&p).contains(".provider(AuthProvider::new(models::Users))"));
        assert!(builder_text(&p).contains(".migrations::<migrations::Migrator>()"));
        assert!(builder_text(&p).contains(".routes(openapi_router())"));
    }

    #[test]
    fn queue_adds_worker_preamble() {
        let p = scaffold(Preset::Minimal, &[Pkg::Queue]);
        assert!(p.preamble.iter().any(|l| l.contains("Worker::builder()")));
        assert!(builder_text(&p).contains(".provider(QueueProvider::new(worker))"));
    }

    #[test]
    fn builder_order_is_correct() {
        let p = scaffold(Preset::Fullstack, &[]);
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
        // Default features, no extras → nothing added to the dependency line.
        assert_eq!(render_feature_list(true, &[]), "");
        assert_eq!(
            render_feature_list(true, &["auth", "openapi"]),
            ", features = [\"auth\", \"openapi\"]"
        );
        // dedups
        assert_eq!(
            render_feature_list(true, &["cache", "cache"]),
            ", features = [\"cache\"]"
        );
        // Non-sqlite driver → defaults off, full list rendered.
        assert_eq!(
            render_feature_list(false, &["orm", "postgres"]),
            ", default-features = false, features = [\"orm\", \"postgres\"]"
        );
    }

    #[test]
    fn sqlite_rides_default_features() {
        let (default_features, features) = driver_features(Db::Sqlite, &["auth", "openapi"]);
        assert!(default_features, "sqlite keeps the crate defaults");
        assert_eq!(
            features,
            vec!["auth", "openapi"],
            "only the package features"
        );
        assert!(
            !features.contains(&"sqlite"),
            "sqlite comes from defaults, not listed"
        );
    }

    #[test]
    fn postgres_turns_off_defaults_and_lists_driver() {
        let (default_features, features) = driver_features(Db::Postgres, &["cache"]);
        assert!(!default_features, "non-sqlite turns defaults off");
        assert!(features.contains(&"postgres"), "lists the chosen driver");
        assert!(
            !features.contains(&"sqlite"),
            "no sqlite alongside postgres"
        );
        // The defaults it replaces are all present so nothing is silently dropped.
        for core in [
            "macros",
            "orm",
            "views",
            "sessions",
            "validation",
            "auth",
            "mail",
            "queue",
        ] {
            assert!(features.contains(&core), "default feature {core} preserved");
        }
        assert!(features.contains(&"cache"), "package feature kept");
    }

    #[test]
    fn env_hints_cover_the_other_drivers() {
        let hints = database_env_hints(Db::Postgres);
        // The active driver is not repeated in the hints.
        assert!(!hints.contains("postgres://"), "active driver not in hints");
        assert!(hints.contains("sqlite://"), "sqlite shown as an option");
        assert!(hints.contains("mysql://"), "mysql shown as an option");
        assert!(hints.contains("features = [\"sqlite\"]"));
        assert!(hints.contains("features = [\"mysql\"]"));
    }

    #[test]
    fn project_name_skips_flag_values() {
        let args = vec![
            "--database".to_owned(),
            "postgres".to_owned(),
            "blog".to_owned(),
        ];
        assert_eq!(project_name(&args).as_deref(), Some("blog"));
        // Name-first form still works.
        let args2 = vec!["blog".to_owned(), "--preset".to_owned(), "api".to_owned()];
        assert_eq!(project_name(&args2).as_deref(), Some("blog"));
    }

    #[test]
    fn database_parse_accepts_aliases() {
        assert_eq!(Db::parse("pg"), Some(Db::Postgres));
        assert_eq!(Db::parse("postgresql"), Some(Db::Postgres));
        assert_eq!(Db::parse("mariadb"), Some(Db::Mysql));
        assert_eq!(Db::parse("sqlite"), Some(Db::Sqlite));
        assert_eq!(Db::parse("oracle"), None);
    }
}
