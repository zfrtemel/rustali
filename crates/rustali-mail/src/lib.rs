//! Email for the Rustali web framework.
//!
//! Define a message by implementing [`Mailable`] (subject + HTML/text body), then send
//! it through the registered [`Mailer`]. Three transports: [`SmtpTransport`] for
//! production (feature `smtp`), [`LogTransport`] that writes to `tracing` for
//! development, and [`MemoryTransport`] that records messages for tests.
//!
//! ```ignore
//! struct Welcome { name: String }
//!
//! impl Mailable for Welcome {
//!     fn subject(&self) -> String { "Welcome!".into() }
//!     fn html(&self) -> String { format!("<h1>Hi {}</h1>", self.name) }
//! }
//!
//! let mailer = ctx.mailer()?;
//! mailer.send("user@example.com", &Welcome { name: "Zafer".into() }).await?;
//! ```

mod mailer;
mod message;
mod provider;
mod transport;

pub use mailer::Mailer;
pub use message::{Email, Mailable};
pub use provider::{MailConfig, MailProvider, MailerExt, SharedMailer};
pub use transport::{DeliverFuture, LogTransport, MemoryTransport, Transport};

#[cfg(feature = "smtp")]
pub use transport::SmtpTransport;
