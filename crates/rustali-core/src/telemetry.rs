//! Logging and tracing setup.

use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::LogConfig;

/// Install the global `tracing` subscriber.
///
/// `RUST_LOG` wins over the configured level when set. Calling this twice (for
/// example in tests) is harmless: the second call is ignored.
pub fn init(config: &LogConfig) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(config.level.as_str()));
    let registry = tracing_subscriber::registry().with(filter);

    let result = if config.json {
        registry
            .with(fmt::layer().json().flatten_event(true))
            .try_init()
    } else {
        registry.with(fmt::layer().compact()).try_init()
    };

    if let Err(err) = result {
        tracing::debug!(%err, "tracing subscriber already installed");
    }
}
