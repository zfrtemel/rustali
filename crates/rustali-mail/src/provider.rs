//! Config, the registered mailer, and the provider.

use std::sync::Arc;

use rustali_core::{AppContext, Provider, RegisterFuture, Registry, Result};
use serde::{Deserialize, Serialize};

use crate::{Mailer, transport::LogTransport};

/// Shared mailer stored as a service.
pub type SharedMailer = Arc<Mailer>;

/// `config/mail.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MailConfig {
    /// Transport: `log` (default, writes to tracing), `memory`, or `smtp`.
    pub transport: String,
    /// Default `From` address.
    pub from: String,
    /// SMTP URL when `transport = "smtp"` (`smtp://user:pass@host:port`).
    pub url: String,
}

impl Default for MailConfig {
    fn default() -> Self {
        Self {
            transport: "log".into(),
            from: "no-reply@example.com".into(),
            url: "smtp://localhost:1025".into(),
        }
    }
}

impl MailConfig {
    /// Section name.
    pub const SECTION: &'static str = "mail";
}

/// Registers a [`Mailer`] from `config/mail.toml`.
#[derive(Debug, Default, Clone, Copy)]
pub struct MailProvider;

impl Provider for MailProvider {
    fn name(&self) -> &'static str {
        "mail"
    }

    fn register<'a>(&'a self, registry: &'a mut Registry<'_>) -> RegisterFuture<'a> {
        Box::pin(async move {
            let config: MailConfig = registry.config().section_or_default(MailConfig::SECTION)?;
            let transport: Arc<dyn crate::Transport> = build_transport(&config)?;
            registry.insert(Arc::new(Mailer::new(transport, config.from)));
            Ok(())
        })
    }
}

#[cfg(feature = "smtp")]
fn build_transport(config: &MailConfig) -> Result<Arc<dyn crate::Transport>> {
    match config.transport.as_str() {
        "smtp" => Ok(Arc::new(crate::SmtpTransport::from_url(&config.url)?)),
        "memory" => Ok(Arc::new(crate::MemoryTransport::new())),
        _ => Ok(Arc::new(LogTransport)),
    }
}

#[cfg(not(feature = "smtp"))]
fn build_transport(config: &MailConfig) -> Result<Arc<dyn crate::Transport>> {
    match config.transport.as_str() {
        "smtp" => Err(rustali_core::Error::bad_request(
            "mail transport `smtp` needs the `smtp` feature on rustali-mail",
        )),
        "memory" => Ok(Arc::new(crate::MemoryTransport::new())),
        _ => Ok(Arc::new(LogTransport)),
    }
}

/// `ctx.mailer()` sugar.
pub trait MailerExt {
    /// The registered mailer, or an error when [`MailProvider`] was not added.
    fn mailer(&self) -> Result<SharedMailer>;
}

impl MailerExt for AppContext {
    fn mailer(&self) -> Result<SharedMailer> {
        self.service::<SharedMailer>().map(|arc| (*arc).clone())
    }
}
