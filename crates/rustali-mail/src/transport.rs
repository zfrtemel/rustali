//! Transports: where rendered emails actually go.

use std::{
    future::Future,
    pin::Pin,
    sync::{Mutex, PoisonError},
};

use rustali_core::Result;

use crate::message::Email;

/// Future returned by [`Transport::deliver`].
pub type DeliverFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

/// Somewhere an [`Email`] can be delivered.
pub trait Transport: Send + Sync + 'static {
    /// Deliver one message.
    fn deliver<'a>(&'a self, email: &'a Email) -> DeliverFuture<'a>;
}

/// Logs emails at `info` instead of sending them. The default in development so you can
/// see what would have gone out without an SMTP server.
#[derive(Debug, Default, Clone, Copy)]
pub struct LogTransport;

impl Transport for LogTransport {
    fn deliver<'a>(&'a self, email: &'a Email) -> DeliverFuture<'a> {
        Box::pin(async move {
            tracing::info!(
                to = %email.to,
                from = %email.from,
                subject = %email.subject,
                "email (log transport; not actually sent)"
            );
            Ok(())
        })
    }
}

/// Records delivered emails in memory for assertions in tests.
#[derive(Debug, Default)]
pub struct MemoryTransport {
    sent: Mutex<Vec<Email>>,
}

impl MemoryTransport {
    /// A new, empty transport.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A clone of every email delivered so far.
    #[must_use]
    pub fn sent(&self) -> Vec<Email> {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Number of emails delivered.
    #[must_use]
    pub fn count(&self) -> usize {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

impl Transport for MemoryTransport {
    fn deliver<'a>(&'a self, email: &'a Email) -> DeliverFuture<'a> {
        Box::pin(async move {
            self.sent
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(email.clone());
            Ok(())
        })
    }
}

/// Sends email over SMTP via `lettre` (feature `smtp`).
#[cfg(feature = "smtp")]
pub struct SmtpTransport {
    inner: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
}

#[cfg(feature = "smtp")]
impl SmtpTransport {
    /// Connect to an SMTP server. `url` is a `smtp://` / `smtps://` URL.
    pub fn from_url(url: &str) -> Result<Self> {
        use lettre::{AsyncSmtpTransport, Tokio1Executor};
        let inner = AsyncSmtpTransport::<Tokio1Executor>::from_url(url)
            .map_err(rustali_core::Error::internal)?
            .build();
        Ok(Self { inner })
    }
}

#[cfg(feature = "smtp")]
impl Transport for SmtpTransport {
    fn deliver<'a>(&'a self, email: &'a Email) -> DeliverFuture<'a> {
        Box::pin(async move {
            use lettre::{
                AsyncTransport, Message,
                message::{MultiPart, SinglePart, header::ContentType},
            };

            let mut builder = Message::builder()
                .from(email.from.parse().map_err(rustali_core::Error::internal)?)
                .to(email.to.parse().map_err(rustali_core::Error::internal)?)
                .subject(&email.subject);
            if let Some(reply_to) = &email.reply_to {
                builder =
                    builder.reply_to(reply_to.parse().map_err(rustali_core::Error::internal)?);
            }

            let html = || {
                SinglePart::builder()
                    .header(ContentType::TEXT_HTML)
                    .body(email.html.clone())
            };
            let body = email.text.as_ref().map_or_else(
                || MultiPart::mixed().singlepart(html()),
                |text| {
                    MultiPart::alternative()
                        .singlepart(SinglePart::plain(text.clone()))
                        .singlepart(html())
                },
            );
            let message = builder
                .multipart(body)
                .map_err(rustali_core::Error::internal)?;
            self.inner
                .send(message)
                .await
                .map_err(rustali_core::Error::internal)?;
            Ok(())
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::message::{Email, Mailable};

    struct Welcome;
    impl Mailable for Welcome {
        fn subject(&self) -> String {
            "Welcome".into()
        }
        fn html(&self) -> String {
            "<h1>Hi</h1>".into()
        }
    }

    #[tokio::test]
    async fn memory_transport_records() {
        let transport = MemoryTransport::new();
        let email = Email::render("app@example.com", "user@example.com", &Welcome);
        transport.deliver(&email).await.expect("deliver");
        assert_eq!(transport.count(), 1);
        assert_eq!(transport.sent()[0].subject, "Welcome");
    }
}
