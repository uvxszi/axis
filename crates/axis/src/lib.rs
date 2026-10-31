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
//!
//! The gateway (`axis-gateway`), event system (`axis-events`), and command
//! framework (`axis-commands`) are separate crates that will be re-exported
//! here as they are implemented; see the README and `docs/ARCHITECTURE.md`
//! for the roadmap.
//!
//! ## Status
//!
//! Phases 1 (foundation) and 2 (REST) are implemented: core types, the
//! error system, data models, and a rate-limit-aware REST client with
//! typed request builders. The gateway (`Http` + WebSocket `Client`) and
//! event system arrive in later phases — examples showing a full `Client`
//! API are design targets for those phases, not working code.

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
