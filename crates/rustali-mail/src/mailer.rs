//! The [`Mailer`]: renders a [`Mailable`] and hands it to the [`Transport`].

use std::sync::Arc;

use rustali_core::Result;

use crate::{Email, message::Mailable, transport::Transport};

/// Sends mail through a configured transport, with a default `from` address.
#[derive(Clone)]
pub struct Mailer {
    transport: Arc<dyn Transport>,
    from: String,
}

impl Mailer {
    /// Build a mailer over `transport` with a default sender address.
    #[must_use]
    pub fn new(transport: Arc<dyn Transport>, from: impl Into<String>) -> Self {
        Self {
            transport,
            from: from.into(),
        }
    }

    /// Render `mail` and send it to `to` from the default address.
    pub async fn send(&self, to: &str, mail: &dyn Mailable) -> Result<()> {
        let email = Email::render(&self.from, to, mail);
        self.transport.deliver(&email).await
    }

    /// Send with an explicit `from`, overriding the default.
    pub async fn send_from(&self, from: &str, to: &str, mail: &dyn Mailable) -> Result<()> {
        let email = Email::render(from, to, mail);
        self.transport.deliver(&email).await
    }

    /// The default sender address.
    #[must_use]
    pub fn from_address(&self) -> &str {
        &self.from
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::transport::MemoryTransport;

    struct Note;
    impl Mailable for Note {
        fn subject(&self) -> String {
            "Note".into()
        }
        fn html(&self) -> String {
            "<p>hi</p>".into()
        }
    }

    #[tokio::test]
    async fn sends_with_default_from() {
        let transport = Arc::new(MemoryTransport::new());
        let mailer = Mailer::new(transport.clone(), "app@example.com");
        mailer.send("user@example.com", &Note).await.expect("send");
        let sent = transport.sent();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].from, "app@example.com");
        assert_eq!(sent[0].to, "user@example.com");
        assert_eq!(sent[0].subject, "Note");
    }
}
