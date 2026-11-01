//! Background jobs for the Rustali web framework.
//!
//! Define work with the [`Job`] trait, dispatch it from a handler, and let a worker
//! run it off the request path:
//!
//! ```ignore
//! #[derive(Serialize, Deserialize)]
//! struct SendWelcome { user_id: i64 }
//!
//! #[async_trait]
//! impl Job for SendWelcome {
//!     const NAME: &'static str = "send_welcome";
//!     async fn handle(&self, ctx: &AppContext) -> Result<()> {
//!         ctx.mailer()?.send(/* ... */).await
//!     }
//! }
//!
//! // in a handler:
//! queue.dispatch(SendWelcome { user_id: user.id }).await?;
//! ```
//!
//! This ships an in-process worker (a bounded channel drained by a Tokio task) — jobs
//! survive the request but not a restart. A durable database/Redis backend plugs in
//! behind the same [`Queue`] trait later; application code does not change.

mod job;
mod memory;
mod provider;
mod worker;

pub use async_trait::async_trait;
pub use job::{Job, JobError, Retry};
pub use memory::MemoryQueue;
pub use provider::{QueueConfig, QueueExt, QueueProvider, SharedQueue};
pub use worker::{Queue, Worker};
