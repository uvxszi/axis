//! # axis-rest
//!
//! Asynchronous REST client for the Discord API.
//!
//! ## Authentication
//!
//! The client authenticates every request with a bot token wrapped in a
//! redacting [`Token`](axis_core::Token) — the token never appears in
//! logs, error messages, or `Debug` output:
//!
//! ```
//! use axis_core::Token;
//! use axis_rest::Http;
//!
//! # fn demo() -> axis_core::Result<()> {
//! let http = Http::builder(Token::from_env("DISCORD_TOKEN")?).build()?;
//! # let _ = http;
//! # Ok(())
//! # }
//! ```
//!
//! ## Rate limits
//!
//! Requests go through a pipeline that tracks Discord's per-bucket limits
//! (learned from `X-RateLimit-*` headers, split by major parameter), waits
//! for exhausted buckets, and waits out 429 responses using the body's
//! `retry_after`. Retries are always bounded; see
//! [`HttpBuilder::max_rate_limit_retries`] and
//! [`HttpBuilder::max_retries`].
//!
//! ## Errors
//!
//! Non-success responses map to typed errors: 401 →
//! [`AxisError::Auth`](axis_core::AuthError), 429 (after bounded retries)
//! → [`AxisError::RateLimit`](axis_core::RateLimitError), other statuses →
//! [`AxisError::Http`](axis_core::HttpError) with the Discord error code
//! and message parsed from the body.
//!
//! ## Endpoints
//!
//! Endpoints are methods on [`Http`] returning awaitable builders, e.g.
//! [`Http::create_message`] → [`CreateMessage`](builders::CreateMessage).
//! Implemented in this phase: messages (create/edit/delete/get/list/bulk
//! delete), channels (get/create/edit/delete/permission overwrites),
//! guilds (get), members (get/kick/ban/unban/edit incl. timeouts), roles
//! (create/edit/delete), and reactions (add/remove/clear).

mod client;
mod endpoints;
mod ratelimit;
mod routing;

pub mod builders;

pub use builders::*;
pub use client::{API_BASE, Http, HttpBuilder};

#[cfg(test)]
pub(crate) mod test_support;
