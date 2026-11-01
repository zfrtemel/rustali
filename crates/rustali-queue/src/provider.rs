//! Config, the registered queue/worker, the provider and the `queue:work` command.

use std::sync::Arc;

use rustali_core::{
    AppContext, Command, CommandFuture, Error, Provider, RegisterFuture, Registry, Result,
};
use serde::{Deserialize, Serialize};

use crate::{MemoryQueue, Worker};

/// Shared queue handle stored as a service. Handlers dispatch through it.
pub type SharedQueue = Arc<MemoryQueue>;

/// `config/queue.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QueueConfig {
    /// Channel capacity for the in-process queue.
    pub capacity: usize,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self { capacity: 1024 }
    }
}

impl QueueConfig {
    /// Section name.
    pub const SECTION: &'static str = "queue";
}

/// Registers the in-process queue and the `queue:work` command.
///
/// The application supplies a [`Worker`] with its jobs registered; the provider stores
/// it alongside the queue so `queue:work` can drain to it.
///
/// ```ignore
/// let worker = Worker::builder().register::<SendWelcome>().build();
/// App::builder().provider(QueueProvider::new(worker)).run().await
/// ```
pub struct QueueProvider {
    worker: Worker,
}

impl QueueProvider {
    /// Use `worker` (built with every job registered).
    #[must_use]
    pub const fn new(worker: Worker) -> Self {
        Self { worker }
    }
}

impl Provider for QueueProvider {
    fn name(&self) -> &'static str {
        "queue"
    }

    fn register<'a>(&'a self, registry: &'a mut Registry<'_>) -> RegisterFuture<'a> {
        Box::pin(async move {
            let config: QueueConfig = registry.config().section_or_default(QueueConfig::SECTION)?;
            registry.insert(Arc::new(MemoryQueue::new(config.capacity)));
            registry.insert(self.worker.clone());
            registry.command(WorkCommand);
            Ok(())
        })
    }
}

/// `queue:work` — drain the in-process queue until Ctrl-C.
struct WorkCommand;

impl Command for WorkCommand {
    fn name(&self) -> &'static str {
        "queue:work"
    }

    fn about(&self) -> &'static str {
        "Process queued jobs until interrupted"
    }

    fn run<'a>(&'a self, ctx: &'a AppContext, _args: Vec<String>) -> CommandFuture<'a> {
        Box::pin(async move {
            let queue = ctx.service::<SharedQueue>()?;
            let worker = ctx.service::<Worker>()?;
            let handle = queue.spawn((*worker).clone(), ctx.clone()).await?;
            println!(
                "Processing jobs ({} handler(s)). Press Ctrl-C to stop.",
                worker.handler_names().len()
            );
            tokio::signal::ctrl_c().await.map_err(Error::internal)?;
            handle.abort();
            println!("\nWorker stopped.");
            Ok(())
        })
    }
}

/// `ctx.queue()` sugar.
pub trait QueueExt {
    /// The registered queue, or an error when [`QueueProvider`] was not added.
    fn queue(&self) -> Result<SharedQueue>;
}

impl QueueExt for AppContext {
    fn queue(&self) -> Result<SharedQueue> {
        self.service::<SharedQueue>().map(|arc| (*arc).clone())
    }
}
