//! The [`Queue`] trait, the handler registry and the [`Worker`] run loop.

use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, time::Duration};

use rustali_core::{AppContext, Error, Result};

use crate::job::{Job, JobError};

/// A serialised job: its handler name and JSON payload, plus retry policy.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Envelope {
    /// Handler name (`Job::NAME`).
    pub name: String,
    /// JSON-encoded job.
    pub payload: String,
    /// Attempts already made.
    pub attempts: u32,
    /// Max additional attempts.
    pub max_attempts: u32,
    /// Backoff between attempts, in milliseconds.
    pub backoff_ms: u64,
}

impl Envelope {
    fn of<J: Job>(job: &J) -> Result<Self> {
        Ok(Self {
            name: J::NAME.to_owned(),
            payload: serde_json::to_string(job).map_err(Error::internal)?,
            attempts: 0,
            max_attempts: J::RETRY.max_attempts,
            backoff_ms: u64::try_from(J::RETRY.backoff.as_millis()).unwrap_or(u64::MAX),
        })
    }
}

/// Where dispatched jobs go. Implemented by [`crate::MemoryQueue`] and (later) durable
/// backends.
pub trait Queue: Send + Sync + 'static {
    /// Enqueue a raw envelope.
    fn push(&self, envelope: Envelope) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;

    /// Enqueue a typed job.
    fn dispatch<'a, J: Job>(
        &'a self,
        job: J,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
    where
        Self: Sized,
    {
        Box::pin(async move {
            let envelope = Envelope::of(&job)?;
            tracing::debug!(job = J::NAME, "dispatching");
            self.push(envelope).await
        })
    }
}

/// Boxed handler that deserialises an envelope and runs the job.
type Handler = Arc<
    dyn Fn(String, AppContext) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync,
>;

/// Runs jobs: holds the handler registry and drains a queue.
///
/// Register every job type with [`Worker::register`], then [`Worker::run`] in a Tokio
/// task (the `queue:work` command does this). The worker owns the [`AppContext`] so
/// handlers can reach services.
#[derive(Clone)]
pub struct Worker {
    handlers: Arc<HashMap<&'static str, Handler>>,
}

/// Builder for [`Worker`].
#[derive(Default)]
pub struct WorkerBuilder {
    handlers: HashMap<&'static str, Handler>,
}

impl Worker {
    /// Start building a worker.
    #[must_use]
    pub fn builder() -> WorkerBuilder {
        WorkerBuilder::default()
    }

    /// Run one envelope to completion, applying its retry policy. Returns `Ok(())`
    /// when the job eventually succeeds or is exhausted; the outcome is logged.
    pub async fn run_once(&self, ctx: &AppContext, mut envelope: Envelope) -> Result<()> {
        let Some(handler) = self.handlers.get(envelope.name.as_str()).cloned() else {
            return Err(Error::internal(JobError(envelope.name)));
        };
        loop {
            match handler(envelope.payload.clone(), ctx.clone()).await {
                Ok(()) => return Ok(()),
                Err(err) if envelope.attempts < envelope.max_attempts => {
                    envelope.attempts += 1;
                    tracing::warn!(
                        job = %envelope.name,
                        attempt = envelope.attempts,
                        %err,
                        "job failed; retrying"
                    );
                    tokio::time::sleep(Duration::from_millis(envelope.backoff_ms)).await;
                }
                Err(err) => {
                    tracing::error!(job = %envelope.name, %err, "job failed; giving up");
                    return Err(err);
                }
            }
        }
    }

    /// Names of every registered handler.
    #[must_use]
    pub fn handler_names(&self) -> Vec<&'static str> {
        self.handlers.keys().copied().collect()
    }
}

impl WorkerBuilder {
    /// Register a job type.
    #[must_use]
    pub fn register<J: Job>(mut self) -> Self {
        let handler: Handler = Arc::new(|payload: String, ctx: AppContext| {
            Box::pin(async move {
                let job: J = serde_json::from_str(&payload).map_err(Error::internal)?;
                job.handle(&ctx).await
            })
        });
        self.handlers.insert(J::NAME, handler);
        self
    }

    /// Finish building.
    #[must_use]
    pub fn build(self) -> Worker {
        Worker {
            handlers: Arc::new(self.handlers),
        }
    }
}
