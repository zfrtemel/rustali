//! In-process queue: a bounded channel drained by a worker task.

use std::{future::Future, pin::Pin};

use rustali_core::{AppContext, Error, Result};
use tokio::sync::{Mutex, mpsc};

use crate::worker::{Envelope, Queue, Worker};

/// An in-memory queue. Dispatched jobs are sent over a channel; [`MemoryQueue::spawn`]
/// starts a background task that drains them through a [`Worker`].
///
/// Jobs survive the request but not a process restart. Fine for development and
/// single-node apps; swap in a durable backend for production fan-out.
pub struct MemoryQueue {
    sender: mpsc::Sender<Envelope>,
    receiver: Mutex<Option<mpsc::Receiver<Envelope>>>,
}

impl MemoryQueue {
    /// Create a queue with a bounded capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity.max(1));
        Self {
            sender,
            receiver: Mutex::new(Some(receiver)),
        }
    }

    /// Start draining the queue in a background task. Takes the receiver, so this may
    /// be called once; subsequent calls return an error.
    pub async fn spawn(
        &self,
        worker: Worker,
        ctx: AppContext,
    ) -> Result<tokio::task::JoinHandle<()>> {
        let mut guard = self.receiver.lock().await;
        let mut receiver = guard
            .take()
            .ok_or_else(|| Error::internal(AlreadyRunning))?;
        drop(guard);

        Ok(tokio::spawn(async move {
            while let Some(envelope) = receiver.recv().await {
                let name = envelope.name.clone();
                if let Err(err) = worker.run_once(&ctx, envelope).await {
                    tracing::error!(job = %name, %err, "job permanently failed");
                }
            }
            tracing::debug!("queue drained; worker stopping");
        }))
    }
}

impl Queue for MemoryQueue {
    fn push(&self, envelope: Envelope) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            self.sender
                .send(envelope)
                .await
                .map_err(|_| Error::internal(QueueClosed))
        })
    }
}

/// Raised when `spawn` is called more than once.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("queue worker already running")]
struct AlreadyRunning;

/// Raised when dispatching to a closed queue.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("queue is closed")]
struct QueueClosed;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicU32, Ordering},
        },
        time::Duration,
    };

    use rustali_core::{App, Config, Result};
    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::{Job, async_trait, job::Retry};

    // A job that increments a shared counter stored as a service.
    #[derive(Serialize, Deserialize)]
    struct Increment;

    #[async_trait]
    impl Job for Increment {
        const NAME: &'static str = "increment";
        async fn handle(&self, ctx: &AppContext) -> Result<()> {
            ctx.service::<Arc<AtomicU32>>()?
                .fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    // A job that fails until the counter reaches a threshold, to exercise retries.
    #[derive(Serialize, Deserialize)]
    struct FlakyUntil(u32);

    #[async_trait]
    impl Job for FlakyUntil {
        const NAME: &'static str = "flaky";
        const RETRY: Retry = Retry::times(5, Duration::from_millis(1));
        async fn handle(&self, ctx: &AppContext) -> Result<()> {
            let counter = ctx.service::<Arc<AtomicU32>>()?;
            let n = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if n >= self.0 {
                Ok(())
            } else {
                Err(Error::internal(std::io::Error::other("not yet")))
            }
        }
    }

    async fn ctx_with_counter(counter: Arc<AtomicU32>) -> AppContext {
        App::builder()
            .config(Config::default())
            .provide(counter)
            .build()
            .await
            .unwrap()
            .0
    }

    #[tokio::test]
    async fn dispatched_jobs_run_on_the_worker() {
        let counter = Arc::new(AtomicU32::new(0));
        let ctx = ctx_with_counter(counter.clone()).await;
        let queue = MemoryQueue::new(16);
        let worker = Worker::builder()
            .register::<Increment>()
            .register::<FlakyUntil>()
            .build();
        let handle = queue.spawn(worker, ctx).await.unwrap();

        for _ in 0..3 {
            queue.dispatch(Increment).await.unwrap();
        }
        // Give the worker a moment to drain.
        for _ in 0..50 {
            if counter.load(Ordering::SeqCst) == 3 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(counter.load(Ordering::SeqCst), 3);
        handle.abort();
    }

    #[tokio::test]
    async fn failing_jobs_retry_until_they_succeed() {
        let counter = Arc::new(AtomicU32::new(0));
        let ctx = ctx_with_counter(counter.clone()).await;
        let worker = Worker::builder().register::<FlakyUntil>().build();
        // Succeeds on the 3rd attempt.
        let envelope = {
            let q = MemoryQueue::new(1);
            q.dispatch(FlakyUntil(3)).await.unwrap();
            q.receiver
                .lock()
                .await
                .as_mut()
                .unwrap()
                .recv()
                .await
                .unwrap()
        };
        worker.run_once(&ctx, envelope).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn unknown_job_name_errors() {
        let ctx = ctx_with_counter(Arc::new(AtomicU32::new(0))).await;
        let worker = Worker::builder().build();
        let envelope = Envelope {
            name: "nope".into(),
            payload: "null".into(),
            attempts: 0,
            max_attempts: 0,
            backoff_ms: 0,
        };
        assert!(worker.run_once(&ctx, envelope).await.is_err());
    }
}
