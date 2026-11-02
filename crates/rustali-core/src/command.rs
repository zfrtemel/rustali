//! Console commands that run inside the application.
//!
//! The application binary is also its own CLI: `cargo run -- migrate`,
//! `cargo run -- routes`. Packages register commands from their [`crate::Provider`];
//! applications add their own with [`crate::AppBuilder::command`]. The global
//! `rustali` tool simply forwards app-bound commands to the binary.

use std::{fmt::Write as _, future::Future, pin::Pin, sync::Arc};

use crate::{AppContext, Error, Result};

/// Future returned by [`Command::run`].
pub type CommandFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

/// A named console command.
///
/// ```ignore
/// struct Hello;
///
/// impl Command for Hello {
///     fn name(&self) -> &'static str { "hello" }
///     fn about(&self) -> &'static str { "Print a greeting" }
///     fn run<'a>(&'a self, ctx: &'a AppContext, args: Vec<String>) -> CommandFuture<'a> {
///         Box::pin(async move {
///             println!("hello {}", args.first().map_or("world", String::as_str));
///             Ok(())
///         })
///     }
/// }
/// ```
pub trait Command: Send + Sync + 'static {
    /// Name typed on the command line, e.g. `migrate` or `migrate:fresh`.
    fn name(&self) -> &'static str;

    /// One-line description shown by `help`.
    fn about(&self) -> &'static str {
        ""
    }

    /// Execute with the remaining arguments.
    fn run<'a>(&'a self, ctx: &'a AppContext, args: Vec<String>) -> CommandFuture<'a>;
}

/// Ordered command registry.
#[derive(Default, Clone)]
pub struct Commands {
    list: Vec<Arc<dyn Command>>,
}

impl Commands {
    /// Register a command, replacing one with the same name.
    pub fn insert(&mut self, command: impl Command) {
        let name = command.name();
        self.list.retain(|existing| existing.name() != name);
        self.list.push(Arc::new(command));
    }

    /// Find a command by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn Command>> {
        self.list.iter().find(|c| c.name() == name).cloned()
    }

    /// All commands in registration order.
    #[must_use]
    pub fn all(&self) -> &[Arc<dyn Command>] {
        &self.list
    }

    /// Render the `help` listing.
    #[must_use]
    pub fn help(&self, bin: &str) -> String {
        let width = self
            .list
            .iter()
            .map(|c| c.name().len())
            .max()
            .unwrap_or(0)
            .max(5);
        let mut out = format!("Usage: {bin} [COMMAND] [ARGS]...\n\nCommands:\n");
        // Writing into a `String` cannot fail.
        let _ = writeln!(
            out,
            "  {:<width$}  Start the HTTP server (default)",
            "serve"
        );
        for command in &self.list {
            let _ = writeln!(out, "  {:<width$}  {}", command.name(), command.about());
        }
        let _ = writeln!(out, "  {:<width$}  Show this list", "help");
        out
    }
}

impl std::fmt::Debug for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.list.iter().map(|c| c.name()))
            .finish()
    }
}

/// Error for an unknown command name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown command `{0}`; run with `help` to list commands")]
pub struct UnknownCommand(pub String);

impl From<UnknownCommand> for Error {
    fn from(err: UnknownCommand) -> Self {
        Self::bad_request(err.to_string())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    struct Noop(&'static str);
    impl Command for Noop {
        fn name(&self) -> &'static str {
            self.0
        }
        fn about(&self) -> &'static str {
            "does nothing"
        }
        fn run<'a>(&'a self, _: &'a AppContext, _: Vec<String>) -> CommandFuture<'a> {
            Box::pin(async { Ok(()) })
        }
    }

    #[test]
    fn insert_replaces_same_name_and_keeps_order() {
        let mut commands = Commands::default();
        commands.insert(Noop("migrate"));
        commands.insert(Noop("routes"));
        commands.insert(Noop("migrate"));
        let names: Vec<_> = commands.all().iter().map(|c| c.name()).collect();
        assert_eq!(names, ["routes", "migrate"]);
        assert!(commands.get("routes").is_some());
        assert!(commands.get("nope").is_none());
    }

    #[test]
    fn help_lists_everything() {
        let mut commands = Commands::default();
        commands.insert(Noop("migrate"));
        let help = commands.help("app");
        assert!(help.contains("serve"));
        assert!(help.contains("migrate"));
        assert!(help.contains("does nothing"));
    }
}
