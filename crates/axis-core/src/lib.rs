//! # axis-core
//!
//! Foundation types for the Axis Discord library: typed snowflake IDs,
//! RFC 3339 timestamps, gateway intents, permissions, the unified error
//! type, and small utilities for handling secrets and logging.
//!
//! Everything in this crate is deliberately dependency-light and
//! runtime-agnostic: it requires no async runtime, no HTTP client, and no
//! WebSocket implementation, so every other Axis crate (and Axis
//! applications) can depend on it without pulling in heavyweight machinery.
//!
//! ## Overview
//!
//! | Module | Contents |
//! |---|---|
//! | [`id`] | Typed snowflake IDs ([`Id`](id::Id), marker types, per-entity aliases) |
//! | [`timestamp`] | RFC 3339 [`Timestamp`] |
//! | [`intents`] | [`GatewayIntents`] bitset |
//! | [`permissions`] | [`Permissions`] bitset |
//! | [`error`] | [`AxisError`] and the per-subsystem error types |
//! | [`secret`] | Redaction-safe [`Secret`] / [`Token`] wrapper |
//! | [`log`] | Optional default `tracing` subscriber setup |

pub mod error;
pub mod id;
pub mod intents;
pub mod log;
pub mod permissions;
pub mod secret;
pub mod timestamp;

// Root re-exports of the most-used types, so `axis_core::UserId` and
// `axis_core::AxisError` work without knowing the module layout.
pub use crate::{
    error::{
        AuthError, AxisError, ConnectionError, GatewayCloseCode, GatewayError, HttpError,
        InternalError, InvalidDataError, PermissionError, RateLimitError, Result,
        SerializationError,
    },
    id::{
        ApplicationId, AutoModRuleId, ChannelId, EmojiId, GenericId, GuildId, Id, IntegrationId,
        InteractionId, MessageId, RoleId, ScheduledEventId, StickerId, UserId, WebhookId, marker,
    },
    intents::GatewayIntents,
    permissions::Permissions,
    secret::{Secret, Token},
    timestamp::Timestamp,
};

/// Convenience re-exports of the most commonly used core types.
pub mod prelude {
    pub use crate::{
        error::{
            AuthError, AxisError, ConnectionError, GatewayCloseCode, GatewayError, HttpError,
            InternalError, InvalidDataError, PermissionError, RateLimitError, Result,
            SerializationError,
        },
        id::{
            ApplicationId, AutoModRuleId, ChannelId, EmojiId, GenericId, GuildId, Id,
            IntegrationId, InteractionId, MessageId, RoleId, ScheduledEventId, StickerId, UserId,
            WebhookId, marker,
        },
        intents::GatewayIntents,
        permissions::Permissions,
        secret::{Secret, Token},
        timestamp::Timestamp,
    };
}
