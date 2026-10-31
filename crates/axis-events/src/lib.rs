//! # axis-events
//!
//! Typed event parsing and asynchronous handler dispatch for the Axis
//! Discord library.
//!
//! Gateway dispatches arrive from [`axis_gateway`] as raw payloads; this
//! crate parses them into the typed [`Event`] enum (models from
//! `axis-model`) and routes them to handlers registered on a
//! [`Dispatcher`]:
//!
//! ```
//! use axis_core::{AxisError, Result};
//! use axis_events::{Dispatcher, Event};
//!
//! # fn demo() -> Dispatcher {
//! let mut dispatcher = Dispatcher::new();
//! dispatcher
//!     .on_message_create(|message| async move {
//!         if message.content == "!hello" {
//!             tracing::info!("hello from {}", message.author.username);
//!         }
//!         Ok(())
//!     })
//!     .on_any(|event| async move {
//!         tracing::trace!(event = ?std::mem::discriminant(&event), "event");
//!         drop(event);
//!         Ok(())
//!     });
//! # dispatcher
//! # }
//! ```
//!
//! ## Semantics
//!
//! - Typed handlers run in registration order per event; the catch-all
//!   (`on_any`) runs first.
//! - Handler errors stop later handlers for that event and are returned
//!   from [`Dispatcher::dispatch`]; the [`Dispatcher::run`] loop logs them
//!   and keeps the bot alive.
//! - Event kinds Axis does not model yet surface as [`Event::Unknown`]
//!   with the raw payload instead of failing — register `on_unknown` to
//!   observe them.
//! - Gateway stream errors are fatal and end [`Dispatcher::run`] with the
//!   error; parse failures for known kinds are logged and skipped.

pub mod dispatcher;
pub mod event;

pub use dispatcher::Dispatcher;
pub use event::Event;
