//! # axis
//!
//! An independent, general-purpose Discord API library for Rust.
//!
//! Axis is a foundational library for building Discord bots, moderation
//! systems, community automation, and similar applications on top of a
//! reliable, typed, async foundation.
//!
//! ## Crate layout
//!
//! The library is a Cargo workspace. This crate is the facade that
//! re-exports the pieces applications actually touch, so depending on
//! `axis` alone is enough:
//!
//! | Module | Crate | Contents |
//! |---|---|---|
//! | [`error`], [`id`], [`intents`], [`permissions`], [`secret`], [`timestamp`], [`log`] | `axis-core` | Foundation types |
//! | [`model`] | `axis-model` | Discord data models |
//! | [`rest`] | `axis-rest` | Asynchronous REST client |
//! | [`gateway`] | `axis-gateway` | Gateway WebSocket shards |
//! | [`events`] | `axis-events` | Typed event parsing + dispatch |
//! | [`commands`] | `axis-commands` | Slash-command framework |
//!
//! Tying it all together: [`client::Client`] — the README's design
//! target, now implemented.
//!
//! ## Status
//!
//! Phases 1 (foundation) through 5 (commands) are implemented: core
//! types, the error system, data models, a rate-limit-aware REST client,
//! a reconnecting/resuming gateway shard, typed event parsing with a
//! handler dispatcher, and a slash-command framework. Phase 6 brings the
//! example bots and long-run polish.

pub mod events {
    //! Typed event parsing and asynchronous handler dispatch.
    pub use axis_events::*;
}

pub mod gateway {
    //! Discord Gateway (WebSocket) shards.
    pub use axis_gateway::*;
}

/// The client: REST + gateway + event dispatch, tied together.
pub mod client;

pub mod commands {
    //! Slash-command framework.
    pub use axis_commands::*;
}

pub mod error {
    //! Unified error types.
    pub use axis_core::error::*;
}

pub mod id {
    //! Typed snowflake IDs, marker types, and per-entity aliases.
    pub use axis_core::id::*;
}

pub mod intents {
    //! Gateway intent bitsets.
    pub use axis_core::intents::*;
}

pub mod log {
    //! Optional default logging setup.
    pub use axis_core::log::*;
}

pub mod model {
    //! Discord data models.
    pub use axis_model::*;
}

pub mod permissions {
    //! Permission bitsets.
    pub use axis_core::permissions::*;
}

pub mod rest {
    //! Asynchronous REST client for the Discord API.
    pub use axis_rest::*;
}

pub mod secret {
    //! Redaction-safe credential wrappers.
    pub use axis_core::secret::*;
}

pub mod timestamp {
    //! RFC 3339 timestamps.
    pub use axis_core::timestamp::*;
}

/// Convenience re-exports of the most commonly used types.
///
/// ```
/// use axis::prelude::*;
///
/// let id = UserId::new(803_511_102_246_789_12);
/// assert_eq!(id.to_string(), "80351110224678912");
/// ```
pub mod prelude {
    pub use crate::{
        client::{Client, ClientBuilder, Context},
        commands::{CommandOption, CommandRegistry, Invocation},
        error::{
            AuthError, AxisError, ConnectionError, GatewayCloseCode, GatewayError, HttpError,
            InternalError, InvalidDataError, PermissionError, RateLimitError, Result,
            SerializationError,
        },
        events::{Dispatcher, Event},
        gateway::{EventStream, Shard, ShardBuilder, ShardEvent},
        id::{
            ApplicationCommandId, ApplicationId, AuditLogEntryId, AutoModRuleId, ChannelId,
            EmojiId, GenericId, GuildId, Id, IntegrationId, InteractionId, MessageId, RoleId,
            ScheduledEventId, StickerId, UserId, WebhookId, marker,
        },
        intents::GatewayIntents,
        log,
        model::{
            Attachment, Channel, ChannelType, Embed, EmbedField, Emoji, Guild, Member, Message,
            MessageFlags, MessageReference, MessageType, PremiumType, Reaction, Role, User,
            UserFlags,
        },
        permissions::Permissions,
        rest::{Http, HttpBuilder},
        secret::{Secret, Token},
        timestamp::Timestamp,
    };
}
