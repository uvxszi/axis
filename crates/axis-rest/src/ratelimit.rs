//! Rate-limit tracking per Discord's rate-limit documentation.
//!
//! Discord keys limits by (bucket hash, major parameter). The bucket hash
//! is learned from the `X-RateLimit-Bucket` response header, so the first
//! request on an unknown route always goes through; every response teaches
//! the limiter the route's bucket and its window state. 429 responses are
//! resolved with `retry_after` (body) or `Retry-After` (header) — never by
//! retrying blindly.
//!
//! Waiting happens outside the lock: [`RateLimiter::acquire`] computes a
//! wait duration under the mutex, sleeps, then re-checks, so a slow waiter
//! never blocks other tasks.

use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard, PoisonError},
    time::{Duration, Instant},
};

use reqwest::header::HeaderMap;
use tracing::debug;

use crate::routing::Route;

/// Window state for one (bucket hash, major parameter) pair.
#[derive(Debug, Clone, Copy)]
struct BucketState {
    limit: u32,
    remaining: u32,
    reset_at: Instant,
}

#[derive(Debug, Default)]
struct State {
    /// Learned mapping: route shape -> Discord bucket hash.
    routes: HashMap<String, String>,
    /// Window state per (bucket hash, major parameter).
    buckets: HashMap<String, BucketState>,
    /// Until when all requests are blocked by a global rate limit.
    global_until: Option<Instant>,
}

/// Tracks Discord's rate limits and decides when requests may be sent.
#[derive(Debug)]
pub(crate) struct RateLimiter {
    state: Mutex<State>,
}

impl RateLimiter {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(State::default()),
        }
    }

    /// Wait until a request on `route` may be sent, then reserve one slot.
    ///
    /// Routes with an unknown bucket are sent immediately: Discord will
    /// answer and teach us the bucket from the response headers.
    pub(crate) async fn acquire(&self, route: &Route) {
        loop {
            let wait = {
                let mut state = self.lock();
                Self::try_acquire(&mut state, route, Instant::now())
            };
            match wait {
                Some(delay) => {
                    debug!(
                        path = %route.path,
                        wait_ms = delay.as_millis() as u64,
                        "waiting for rate-limit bucket"
                    );
                    tokio::time::sleep(delay).await;
                }
                None => return,
            }
        }
    }

    /// Update or reserve bucket state under the lock. Returns the duration
    /// to wait, or `None` if the request may proceed (consuming one slot).
    fn try_acquire(state: &mut State, route: &Route, now: Instant) -> Option<Duration> {
        if let Some(until) = state.global_until {
            if until > now {
                return Some(until.saturating_duration_since(now));
            }
            state.global_until = None;
        }

        let hash = state.routes.get(&route.route_key())?.clone();
        let key = route.bucket_key(&hash);
        let bucket = state.buckets.get_mut(&key)?;

        if now >= bucket.reset_at {
            // The window has rolled over; refill. Response headers will
            // correct this state when they arrive.
            bucket.remaining = bucket.limit;
        }

        if bucket.remaining > 0 {
            bucket.remaining -= 1;
            None
        } else {
            Some(bucket.reset_at.saturating_duration_since(now))
        }
    }

    /// Learn the route's bucket and window state from response headers.
    ///
    /// Absent headers (e.g. on some error responses) leave state untouched.
    pub(crate) fn update_from_headers(&self, route: &Route, headers: &HeaderMap) {
        let Some(hash) = header_str(headers, "x-ratelimit-bucket") else {
            return;
        };
        let remaining = header_u32(headers, "x-ratelimit-remaining");
        let limit = header_u32(headers, "x-ratelimit-limit");
        let reset_after = header_f64(headers, "x-ratelimit-reset-after")
            .filter(|value| value.is_finite() && *value >= 0.0);

        let mut state = self.lock();
        state.routes.insert(route.route_key(), hash.to_owned());
        if let (Some(remaining), Some(reset_after)) = (remaining, reset_after) {
            let Some(reset_after) = Duration::try_from_secs_f64(reset_after).ok() else {
                return;
            };
            let reset_at = Instant::now().checked_add(reset_after);
            let Some(reset_at) = reset_at else { return };
            let key = route.bucket_key(hash);
            state.buckets.insert(
                key,
                BucketState {
                    limit: limit.unwrap_or(remaining),
                    remaining,
                    reset_at,
                },
            );
        }
    }

    /// Block all requests until a global rate limit expires.
    pub(crate) fn penalize_globally(&self, retry_after: Duration) {
        let mut state = self.lock();
        state.global_until = Some(Instant::now() + retry_after);
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        // Recover from poisoning: a panic while holding the lock leaves
        // consistent-ish data behind; retrying is better than panicking
        // every future request.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok()
}

fn header_u32(headers: &HeaderMap, name: &str) -> Option<u32> {
    header_str(headers, name)?.trim().parse().ok()
}

fn header_f64(headers: &HeaderMap, name: &str) -> Option<f64> {
    header_str(headers, name)?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route_get_messages(channel: u64) -> Route {
        Route::with_major(
            reqwest::Method::GET,
            "/channels/{channel_id}/messages",
            format!("/channels/{channel}/messages"),
            channel,
        )
    }

    fn headers_with(bucket: &str, remaining: u32, reset_after: f64) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-bucket", bucket.parse().unwrap());
        headers.insert(
            "x-ratelimit-remaining",
            remaining.to_string().parse().unwrap(),
        );
        headers.insert("x-ratelimit-limit", "5".parse().unwrap());
        headers.insert(
            "x-ratelimit-reset-after",
            reset_after.to_string().parse().unwrap(),
        );
        headers
    }

    #[tokio::test]
    async fn unknown_route_is_allowed_immediately() {
        let limiter = RateLimiter::new();
        // Must not wait: no bucket is known yet.
        tokio::time::timeout(
            std::time::Duration::from_millis(100),
            limiter.acquire(&route_get_messages(1)),
        )
        .await
        .expect("first request on unknown route must not wait");
    }

    #[tokio::test]
    async fn exhausted_bucket_waits_until_reset() {
        let limiter = RateLimiter::new();
        let route = route_get_messages(1);
        limiter.update_from_headers(&route, &headers_with("b1", 0, 0.2));

        let start = Instant::now();
        limiter.acquire(&route).await;
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(180),
            "expected to wait ~200ms, waited {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "waited far too long: {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn buckets_are_split_by_major_parameter() {
        let limiter = RateLimiter::new();
        limiter.update_from_headers(&route_get_messages(1), &headers_with("b1", 0, 5.0));
        // A different channel must not inherit the exhausted bucket.
        tokio::time::timeout(
            Duration::from_millis(100),
            limiter.acquire(&route_get_messages(2)),
        )
        .await
        .expect("different major parameter must not share the exhausted bucket");
    }

    #[tokio::test]
    async fn available_bucket_is_allowed_without_wait() {
        let limiter = RateLimiter::new();
        let route = route_get_messages(1);
        limiter.update_from_headers(&route, &headers_with("b1", 3, 5.0));
        tokio::time::timeout(Duration::from_millis(100), limiter.acquire(&route))
            .await
            .expect("bucket with remaining capacity must not wait");
    }

    #[tokio::test]
    async fn global_penalty_blocks_all_routes() {
        let limiter = RateLimiter::new();
        limiter.penalize_globally(Duration::from_millis(200));
        let start = Instant::now();
        limiter.acquire(&route_get_messages(99)).await;
        assert!(start.elapsed() >= Duration::from_millis(180));
    }

    #[test]
    fn headers_without_bucket_are_ignored() {
        let limiter = RateLimiter::new();
        let route = route_get_messages(1);
        limiter.update_from_headers(&route, &HeaderMap::new());
        // No panic, no learned route.
        assert!(limiter.lock().routes.is_empty());
    }
}
