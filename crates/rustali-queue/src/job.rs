//! The [`Job`] trait and its retry policy.

use std::time::Duration;

use rustali_core::{AppContext, Result};

use crate::async_trait;

/// A unit of background work.
///
/// Jobs are serialised to JSON for the queue, so they must be `Serialize +
/// DeserializeOwned`. `handle` runs on a worker with access to the full
/// [`AppContext`] (database, mailer, cache...).
#[async_trait]
pub trait Job: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static {
    /// Stable name used to route serialised jobs back to their handler. Must be unique.
    const NAME: &'static str;

    /// Retry policy applied when `handle` returns an error.
    const RETRY: Retry = Retry::NONE;

    /// Do the work.
    async fn handle(&self, ctx: &AppContext) -> Result<()>;
}

/// How many times to retry a failed job and how long to wait between attempts
/// (fixed backoff for now; exponential can come later).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retry {
    /// Maximum attempts beyond the first.
    pub max_attempts: u32,
    /// Delay between attempts.
    pub backoff: Duration,
}

impl Retry {
    /// No retries: one attempt, failures are logged and dropped.
    pub const NONE: Self = Self {
        max_attempts: 0,
        backoff: Duration::ZERO,
    };

    /// `n` retries with a fixed `backoff` between them.
    #[must_use]
    pub const fn times(n: u32, backoff: Duration) -> Self {
        Self {
            max_attempts: n,
            backoff,
        }
    }
}

/// Error raised when a serialised job names a handler nobody registered.
#[derive(Debug, Clone, thiserror::Error)]
#[error("no handler registered for job `{0}`")]
pub struct JobError(pub String);
