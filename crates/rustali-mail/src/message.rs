//! The [`Mailable`] trait and the rendered [`Email`].

/// A type that can be turned into an email: a subject and an HTML body, with an
/// optional plain-text alternative.
///
/// Templates (Askama/Tera) fit naturally: render the template to a string in
/// [`Mailable::html`].
pub trait Mailable: Send + Sync {
    /// The subject line.
    fn subject(&self) -> String;

    /// The HTML body.
    fn html(&self) -> String;

    /// An optional plain-text alternative. Defaults to `None`.
    fn text(&self) -> Option<String> {
        None
    }

    /// Reply-To address, if any.
    fn reply_to(&self) -> Option<String> {
        None
    }
}

/// A fully addressed, rendered message, ready for a [`crate::Transport`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email {
    /// Sender (`From`).
    pub from: String,
    /// Recipient (`To`).
    pub to: String,
    /// Subject line.
    pub subject: String,
    /// HTML body.
    pub html: String,
    /// Optional plain-text body.
    pub text: Option<String>,
    /// Optional `Reply-To`.
    pub reply_to: Option<String>,
}

impl Email {
    /// Build an [`Email`] from a [`Mailable`], a sender and a recipient.
    pub fn render(from: &str, to: &str, mail: &dyn Mailable) -> Self {
        Self {
            from: from.to_owned(),
            to: to.to_owned(),
            subject: mail.subject(),
            html: mail.html(),
            text: mail.text(),
            reply_to: mail.reply_to(),
        }
    }
}
