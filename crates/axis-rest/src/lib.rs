//! # axis-rest
//!
//! Asynchronous REST client for the Discord API.
//!
//! ## Planned responsibilities (Phase 2)
//!
//! - Token-based authentication (managed as a redacted [`Secret`-like]
//!   credential that never leaks into logs).
//! - An async HTTP client on top of `reqwest` with typed request and
//!   response models.
//! - Per-route and global rate-limit handling: bucket tracking, honoring
//!   `retry_after` and `X-RateLimit-*` headers, and bounded retries — never
//!   naive unlimited retries.
//! - Message, channel, guild, member, role, reaction, ban, and timeout
//!   operations behind ergonomic request builders.
//! - Consistent mapping of HTTP failures into [`AxisError::Http`],
//!   [`AxisError::RateLimit`], [`AxisError::Permission`], and
//!   [`AxisError::Auth`] (for 401s).
//!
//! ## Status
//!
//! Not yet implemented. This crate intentionally contains no APIs yet —
//! see the repository README and `docs/ARCHITECTURE.md` for the plan.
//!
//! [`AxisError::Http`]: axis_core::error::AxisError::Http
