//! Fixed-window rate limiting.

use std::{
    collections::HashMap,
    future::Future,
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::{Duration, Instant},
};

use axum::{
    extract::{ConnectInfo, Request},
    http::HeaderValue,
    response::{IntoResponse, Response},
};
use tower::{Layer, Service};

use crate::Error;

/// Per-client request limit over a fixed window, in memory.
///
/// Clients are keyed by `X-Forwarded-For` (first address) or the peer address when
/// the server was started with `ConnectInfo`; otherwise all requests share one
/// bucket. Exceeding the limit yields `429 Too Many Requests` with `Retry-After`.
///
/// ```ignore
/// #[controller("/api")]
/// #[middleware(Throttle::per_minute(60))]
/// impl ApiController { /* ... */ }
/// ```
///
/// This is the single-process implementation; a shared store (Redis) arrives with
/// the cache package.
#[derive(Debug, Clone)]
pub struct Throttle {
    max: u32,
    window: Duration,
    buckets: Arc<Mutex<HashMap<String, Bucket>>>,
}

#[derive(Debug, Clone, Copy)]
struct Bucket {
    count: u32,
    started: Instant,
}

/// Outcome of one hit: requests left in the window, or seconds until it resets.
pub type HitResult = Result<u32, u64>;

/// Response header carrying the requests left in the current window.
pub const REMAINING_HEADER: &str = "x-ratelimit-remaining";

impl Throttle {
    /// At most `max` requests per `window`.
    #[must_use]
    pub fn new(max: u32, window: Duration) -> Self {
        Self {
            max: max.max(1),
            window,
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// At most `max` requests per minute.
    #[must_use]
    pub fn per_minute(max: u32) -> Self {
        Self::new(max, Duration::from_secs(60))
    }

    /// At most `max` requests per second.
    #[must_use]
    pub fn per_second(max: u32) -> Self {
        Self::new(max, Duration::from_secs(1))
    }

    /// Record a hit for `key`.
    pub fn hit(&self, key: &str, now: Instant) -> HitResult {
        let mut buckets = self
            .buckets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let outcome = Self::hit_locked(&mut buckets, key, now, self.max, self.window);
        drop(buckets);
        outcome
    }

    fn hit_locked(
        buckets: &mut HashMap<String, Bucket>,
        key: &str,
        now: Instant,
        max: u32,
        window: Duration,
    ) -> HitResult {
        // Opportunistic cleanup keeps the map bounded without a background task.
        if buckets.len() > 10_000 {
            buckets.retain(|_, b| now.duration_since(b.started) < window);
        }
        let bucket = buckets.entry(key.to_owned()).or_insert(Bucket {
            count: 0,
            started: now,
        });
        if now.duration_since(bucket.started) >= window {
            *bucket = Bucket {
                count: 0,
                started: now,
            };
        }
        if bucket.count >= max {
            let reset = window.saturating_sub(now.duration_since(bucket.started));
            return Err(reset.as_secs().max(1));
        }
        bucket.count += 1;
        Ok(max - bucket.count)
    }

    fn key(request: &Request) -> String {
        if let Some(forwarded) = request
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            return forwarded.to_owned();
        }
        request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map_or_else(|| "global".to_owned(), |info| info.0.ip().to_string())
    }
}

impl<S> Layer<S> for Throttle {
    type Service = ThrottleService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ThrottleService {
            inner,
            throttle: self.clone(),
        }
    }
}

/// Service produced by [`Throttle`].
#[derive(Debug, Clone)]
pub struct ThrottleService<S> {
    inner: S,
    throttle: Throttle,
}

impl<S> Service<Request> for ThrottleService<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let key = Throttle::key(&request);
        match self.throttle.hit(&key, Instant::now()) {
            Ok(remaining) => {
                let fut = self.inner.call(request);
                Box::pin(async move {
                    let mut response = fut.await?;
                    // Nested throttles (controller + method) both report; keep the most
                    // restrictive number so clients see the real headroom.
                    let existing = response
                        .headers()
                        .get(REMAINING_HEADER)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u32>().ok());
                    let shown = existing.map_or(remaining, |e| e.min(remaining));
                    if let Ok(value) = HeaderValue::from_str(&shown.to_string()) {
                        response.headers_mut().insert(REMAINING_HEADER, value);
                    }
                    Ok(response)
                })
            }
            Err(retry_after) => Box::pin(async move {
                Ok(Error::TooManyRequests {
                    retry_after_secs: retry_after,
                }
                .into_response())
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_within_window_and_resets_after() {
        let throttle = Throttle::new(2, Duration::from_secs(10));
        let t0 = Instant::now();
        assert_eq!(throttle.hit("a", t0), Ok(1));
        assert_eq!(throttle.hit("a", t0), Ok(0));
        assert_eq!(throttle.hit("a", t0 + Duration::from_secs(3)), Err(7));
        assert_eq!(
            throttle.hit("b", t0),
            Ok(1),
            "other clients are independent"
        );
        assert_eq!(
            throttle.hit("a", t0 + Duration::from_secs(10)),
            Ok(1),
            "window reset"
        );
    }
}
