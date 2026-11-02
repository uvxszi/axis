//! # axis-gateway
//!
//! Discord Gateway (WebSocket) client.
//!
//! A [`Shard`] maintains one gateway connection on a background task: it
//! identifies, heartbeats (with zombie detection), tracks sequence
//! numbers, and reconnects with the correct resume-vs-identify decision.
//! Events arrive through an [`EventStream`]:
//!
//! ```
//! use axis_core::{GatewayIntents, Token};
//! use axis_gateway::{Shard, ShardEvent};
//! # async fn demo() -> axis_core::Result<()> {
//! let shard = Shard::builder(Token::from_env("DISCORD_TOKEN")?, GatewayIntents::unprivileged())
//!     .build()?;
//! let mut events = shard.start();
//!
//! while let Some(event) = events.next_event().await {
//!     match event? {
//!         ShardEvent::Dispatch { kind, data } => {
//!             tracing::debug!(kind, "dispatch received");
//!             let _ = data;
//!         }
//!         ShardEvent::Connected | ShardEvent::Resumed => {}
//!         ShardEvent::Reconnecting { close_code } => {
//!             tracing::warn!(?close_code, "shard reconnecting");
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Reconnect vs resume
//!
//! Per Discord's protocol, reconnecting does not always mean identifying
//! again. Axis decides per trigger:
//!
//! - `Reconnect` (op 7), resumable `Invalid Session` (op 9), transport
//!   loss, zombie heartbeats, and resume-able close codes → send **Resume**
//!   on a new connection to the `resume_gateway_url` from READY.
//! - `Invalid Session` with `d: false`, close codes 1000/1001, and close
//!   codes 4007/4009 → drop the session and **Identify** afresh.
//! - Fatal close codes (4004, 4010-4014 — bad token, bad shard setup,
//!   invalid intents) end the stream with a typed error; retrying cannot
//!   fix those.
//!
//! ## Sharding
//!
//! One `Shard` is one connection. [`ShardBuilder::shard`] configures the
//! shard index/count pair sent in Identify; running multiple shards is a
//! matter of spawning several (a coordinating manager arrives later).
//! Session-start limits from `GET /gateway/bot` are not consulted yet.

mod connection;
mod presence;
mod protocol;
mod session;
mod shard;

pub use presence::{Activity, ActivityKind, OnlineStatus, Presence, ShardMessenger};
pub use protocol::DEFAULT_GATEWAY_URL;
pub use shard::{EventStream, Shard, ShardBuilder, ShardEvent};
