//! # axis-model
//!
//! Strongly typed data models for objects exchanged with the Discord API:
//! users, guilds, channels, messages, roles, and so on.
//!
//! ## Design notes
//!
//! - Discord objects change shape depending on context: the same `User`
//!   object appears in full form in `Message.author`, with extra fields in
//!   gateway `Presence Update`, and trimmed down elsewhere. Optional
//!   Discord fields are therefore modeled as `Option<T>`, and unknown
//!   fields are ignored on deserialization rather than rejected — the API
//!   grows new fields over time, and an unknown field must not break a
//!   running bot.
//! - IDs use the typed [`axis_core::id::Id`] wrapper, so a user ID cannot
//!   be confused with a channel ID.
//! - Bitsets (permissions, flags) deserialize leniently: bits the library
//!   does not know about are preserved where harmless.
//!
//! ## Status
//!
//! Foundation phase: the `User` model establishes the patterns
//! (serialization shape, optional fields, typed IDs, bitset flags) that the
//! remaining models follow. Guild, channel, message, role, and the rest of
//! the model set arrive in the models phase.

pub mod user;

pub use user::{PremiumType, User, UserFlags};
