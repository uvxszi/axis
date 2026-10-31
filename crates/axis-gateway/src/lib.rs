//! # axis-gateway
//!
//! Discord Gateway (WebSocket) client.
//!
//! ## Planned responsibilities (Phase 3)
//!
//! - Connecting to the gateway and authenticating with `Identify`.
//! - Heartbeating and heartbeat acknowledgement (`op 1`/`op 11`) tracking.
//! - Sequence number tracking with correct reconnection semantics:
//!   `Resume` when the session is still valid, a fresh `Identify` after
//!   close codes 4007/4009, and a hard stop for non-reconnectable codes
//!   (4004, 4010-4014). Reconnecting never blindly re-identifies.
//! - Session management with `session_id` and `resume_gateway_url` handling.
//! - Dispatching decoded events with intent-based subscriptions.
//! - Sharding support built into the connection model from the start.
//! - Graceful shutdown without leaking heartbeat or reader tasks.
//!
//! The error types this crate will produce ([`GatewayError`],
//! [`GatewayCloseCode`]) already exist in `axis-core`, including the
//! documented reconnect/resume semantics per close code.
//!
//! ## Status
//!
//! Not yet implemented. This crate intentionally contains no APIs yet —
//! see the repository README and `docs/ARCHITECTURE.md` for the plan.
//!
//! [`GatewayError`]: axis_core::error::GatewayError
//! [`GatewayCloseCode`]: axis_core::error::GatewayCloseCode
