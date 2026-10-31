//! The typed event enum and its parser.

use serde::de::DeserializeOwned;

use axis_core::error::{AxisError, Result, SerializationError};
use axis_model::{
    Channel, Guild, GuildCreate, GuildRoleChange, GuildRoleDelete, Interaction, MemberAdd,
    MemberRemove, MemberUpdate, Message, MessageDelete, MessageDeleteBulk, MessageReactionAdd,
    MessageReactionRemove, PartialMessage, Ready, UnavailableGuild,
};

/// A parsed gateway event.
///
/// Payloads are the typed models from `axis-model`. Event kinds Discord
/// adds after this library was written surface as
/// [`Event::Unknown`] instead of failing — a running bot must survive API
/// growth. Parse failures for *known* kinds surface as errors from
/// [`Event::parse`]; the dispatcher's `run` loop logs and skips them.
///
/// The size difference between variants is deliberate: events are
/// short-lived values passed by move to handlers, and boxing payloads
/// would make handler signatures awkward for no measurable gain.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum Event {
    /// `READY`: the session started; carries the bot's user and session
    /// identity.
    Ready(Ready),
    /// `RESUMED`: a resumed session is live again; missed events were
    /// replayed before this.
    Resumed,
    /// `MESSAGE_CREATE`.
    MessageCreate(Message),
    /// `MESSAGE_UPDATE`: a partial message.
    MessageUpdate(PartialMessage),
    /// `MESSAGE_DELETE`.
    MessageDelete(MessageDelete),
    /// `MESSAGE_DELETE_BULK`.
    MessageDeleteBulk(MessageDeleteBulk),
    /// `MESSAGE_REACTION_ADD`.
    ReactionAdd(MessageReactionAdd),
    /// `MESSAGE_REACTION_REMOVE`.
    ReactionRemove(MessageReactionRemove),
    /// `GUILD_MEMBER_ADD`.
    MemberAdd(MemberAdd),
    /// `GUILD_MEMBER_UPDATE`.
    MemberUpdate(MemberUpdate),
    /// `GUILD_MEMBER_REMOVE`.
    MemberRemove(MemberRemove),
    /// `GUILD_CREATE`: a guild became available or the bot joined it.
    GuildCreate(GuildCreate),
    /// `GUILD_UPDATE`.
    GuildUpdate(Guild),
    /// `GUILD_DELETE`: the guild is unavailable (outage) or the bot left
    /// or was removed.
    GuildDelete(UnavailableGuild),
    /// `CHANNEL_CREATE`.
    ChannelCreate(Channel),
    /// `CHANNEL_UPDATE`.
    ChannelUpdate(Channel),
    /// `CHANNEL_DELETE`.
    ChannelDelete(Channel),
    /// `GUILD_ROLE_CREATE`.
    RoleCreate(GuildRoleChange),
    /// `GUILD_ROLE_UPDATE`.
    RoleUpdate(GuildRoleChange),
    /// `GUILD_ROLE_DELETE`.
    RoleDelete(GuildRoleDelete),
    /// `INTERACTION_CREATE`: command invocations, component presses,
    /// autocomplete requests, and modal submissions.
    InteractionCreate(Interaction),
    /// The shard lost its connection and is re-establishing it. The happy
    /// paths surface as [`Event::Ready`] / [`Event::Resumed`]; this event
    /// covers the gap between a disconnect and the next READY so
    /// applications can show reconnect state.
    Reconnecting {
        /// The close code the server sent, when the disconnect carried one.
        close_code: Option<u16>,
    },
    /// An event kind this version of Axis does not model yet. The raw
    /// payload is preserved; forward it (or ignore it) as appropriate.
    Unknown {
        /// The gateway event name (`t`).
        kind: String,
        /// The raw payload (`d`).
        data: serde_json::Value,
    },
}

impl Event {
    /// Parse a dispatch (`t` + `d`) into a typed event.
    ///
    /// Kinds this library does not model become [`Event::Unknown`]; known
    /// kinds whose payload fails to deserialize return a
    /// [`SerializationError`].
    pub fn parse(kind: &str, data: serde_json::Value) -> Result<Self> {
        let event = match kind {
            "READY" => Self::Ready(parse(data)?),
            "RESUMED" => Self::Resumed,
            "MESSAGE_CREATE" => Self::MessageCreate(parse(data)?),
            "MESSAGE_UPDATE" => Self::MessageUpdate(parse(data)?),
            "MESSAGE_DELETE" => Self::MessageDelete(parse(data)?),
            "MESSAGE_DELETE_BULK" => Self::MessageDeleteBulk(parse(data)?),
            "MESSAGE_REACTION_ADD" => Self::ReactionAdd(parse(data)?),
            "MESSAGE_REACTION_REMOVE" => Self::ReactionRemove(parse(data)?),
            "GUILD_MEMBER_ADD" => Self::MemberAdd(parse(data)?),
            "GUILD_MEMBER_UPDATE" => Self::MemberUpdate(parse(data)?),
            "GUILD_MEMBER_REMOVE" => Self::MemberRemove(parse(data)?),
            "GUILD_CREATE" => Self::GuildCreate(parse(data)?),
            "GUILD_UPDATE" => Self::GuildUpdate(parse(data)?),
            "GUILD_DELETE" => Self::GuildDelete(parse(data)?),
            "CHANNEL_CREATE" => Self::ChannelCreate(parse(data)?),
            "CHANNEL_UPDATE" => Self::ChannelUpdate(parse(data)?),
            "CHANNEL_DELETE" => Self::ChannelDelete(parse(data)?),
            "GUILD_ROLE_CREATE" => Self::RoleCreate(parse(data)?),
            "GUILD_ROLE_UPDATE" => Self::RoleUpdate(parse(data)?),
            "GUILD_ROLE_DELETE" => Self::RoleDelete(parse(data)?),
            "INTERACTION_CREATE" => Self::InteractionCreate(parse(data)?),
            other => Self::Unknown {
                kind: other.to_owned(),
                data,
            },
        };
        Ok(event)
    }
}

fn parse<T: DeserializeOwned>(data: serde_json::Value) -> Result<T> {
    serde_json::from_value(data)
        .map_err(|error| AxisError::Serialization(SerializationError::Json(error)))
}
