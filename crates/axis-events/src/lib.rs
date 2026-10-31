//! # axis-events
//!
//! Flexible asynchronous event dispatch for the Axis Discord library.
//!
//! ## Planned responsibilities (Phase 4)
//!
//! - Parsing raw gateway payloads into typed models.
//! - An event enum covering `MessageCreate`, `MessageUpdate`,
//!   `MessageDelete`, member join/leave/update, guild/channel/role
//!   lifecycle events, `InteractionCreate`, and reaction events.
//! - Handler registration with an ergonomic public API (the
//!   `client.on_message_create(|ctx, message| ...)` style from the README's
//!   design target), preserving Rust type safety.
//! - Handler concurrency model: handlers run as async tasks with
//!   predictable cancellation on shutdown.
//!
//! ## Status
//!
//! Not yet implemented. This crate intentionally contains no APIs yet —
//! see the repository README and `docs/ARCHITECTURE.md` for the plan.
