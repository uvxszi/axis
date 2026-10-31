//! The Discord interaction object.
//!
//! The `Interaction` envelope is fully typed; `data` is decoded per
//! interaction type into [`InteractionData`].

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use axis_core::{
    ApplicationCommandId, ApplicationId, ChannelId, GuildId, InteractionId, Permissions,
    id::GenericId,
};

use crate::{member::Member, message::Message, role::Role, user::User};

api_enum! {
    /// The type of an interaction.
    InteractionType: u8 {
        /// A Ping (webhook verification), never sent over the gateway.
        Ping = 1,
        /// A slash command or context-menu command invocation.
        ApplicationCommand = 2,
        /// A message-component press.
        MessageComponent = 3,
        /// An autocomplete request for a command option.
        ApplicationCommandAutocomplete = 4,
        /// A modal submission.
        ModalSubmit = 5,
    }
}

api_enum! {
    /// The type of an interaction response (callback), from the
    /// documented Interaction Callback Type table.
    InteractionCallbackType: u8 {
        /// Acknowledge a Ping.
        Pong = 1,
        /// Respond with a message; the user sees the command resolve.
        ChannelMessageWithSource = 4,
        /// Acknowledge and respond later; the user sees a loading state.
        DeferredChannelMessageWithSource = 5,
        /// Acknowledge a component press and edit the source message later.
        DeferredUpdateMessage = 6,
        /// Edit the message a component was attached to.
        UpdateMessage = 7,
        /// Respond to an autocomplete request with suggested choices.
        ApplicationCommandAutocompleteResult = 8,
        /// Respond with a popup modal.
        Modal = 9,
        /// Deprecated premium upgrade-button response.
        PremiumRequired = 10,
        /// Launch the application's Activity.
        LaunchActivity = 12,
    }
}

/// The message-style callback data accepted by message responses and
/// followups. Only the documented flags may be set here (ephemeral,
/// suppress embeds, ...).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InteractionMessageCallbackData {
    /// Whether to send with text-to-speech.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<bool>,
    /// The message content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// The embeds to attach (up to 10).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Vec<crate::message::Embed>>,
    /// Message flags; only a subset may be set here (e.g.
    /// [`crate::message::MessageFlags::EPHEMERAL`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<crate::message::MessageFlags>,
}

/// The typed `data` of an interaction, decoded according to the
/// interaction type.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum InteractionData {
    /// An application-command invocation (slash or context menu).
    ApplicationCommand(Box<CommandData>),
    /// An autocomplete request for a command option.
    Autocomplete(Box<CommandData>),
    /// A message-component press.
    MessageComponent(Box<ComponentData>),
    /// A modal submission; `components` stays raw until component models
    /// land.
    ModalSubmit(Box<ModalSubmitData>),
}

/// The `data` of a message-component interaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentData {
    /// The developer-defined custom ID of the pressed component.
    pub custom_id: String,
    /// The documented component type value (buttons, select menus, ...).
    pub component_type: u8,
    /// The selected values, for select menus.
    pub values: Option<Vec<String>>,
}

/// The `data` of a modal submission. Components stay raw until component
/// models land.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModalSubmitData {
    /// The developer-defined custom ID of the submitted modal.
    pub custom_id: String,
    /// The submitted components (raw).
    pub components: Value,
}

/// An interaction: a command invocation, component press, autocomplete
/// request, or modal submission.
#[derive(Debug, Clone, PartialEq)]
pub struct Interaction {
    /// The interaction ID.
    pub id: InteractionId,
    /// The application the interaction is for.
    pub application_id: ApplicationId,
    /// The interaction type.
    pub r#type: InteractionType,
    /// The typed interaction data, decoded per [`Self::r#type`].
    pub data: Option<InteractionData>,
    /// The guild it was invoked in, when applicable.
    pub guild_id: Option<GuildId>,
    /// The channel it was invoked in, when applicable.
    pub channel_id: Option<ChannelId>,
    /// The invoking member, when invoked in a guild.
    pub member: Option<Member>,
    /// The invoking user, when invoked in a DM.
    pub user: Option<User>,
    /// The continuation token for responding (valid 15 minutes; an initial
    /// response is due within 3 seconds).
    pub token: String,
    /// The interaction version; always 1.
    pub version: u8,
    /// The invoking user's locale, when applicable.
    pub locale: Option<String>,
    /// The guild's preferred locale, when applicable.
    pub guild_locale: Option<String>,
    /// The message a component or modal interaction is attached to.
    pub message: Option<Message>,
    /// The bot's effective permissions at the invocation site.
    pub app_permissions: Option<Permissions>,
    /// The maximum attachment size the invoking user may upload.
    pub attachment_size_limit: Option<u64>,
}

impl Interaction {
    /// The command data, when this is a command invocation or autocomplete
    /// request.
    pub fn command_data(&self) -> Option<&CommandData> {
        match self.data.as_ref() {
            Some(InteractionData::ApplicationCommand(data)) => Some(data),
            Some(InteractionData::Autocomplete(data)) => Some(data),
            _ => None,
        }
    }
}

impl Serialize for Interaction {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        /// The wire envelope: `data` is raw, decoded per interaction type.
        #[derive(Serialize)]
        struct RawInteraction<'a> {
            id: &'a InteractionId,
            application_id: &'a ApplicationId,
            r#type: InteractionType,
            #[serde(skip_serializing_if = "Option::is_none")]
            data: Option<Value>,
            #[serde(skip_serializing_if = "Option::is_none")]
            guild_id: &'a Option<GuildId>,
            #[serde(skip_serializing_if = "Option::is_none")]
            channel_id: &'a Option<ChannelId>,
            #[serde(skip_serializing_if = "Option::is_none")]
            member: &'a Option<Member>,
            #[serde(skip_serializing_if = "Option::is_none")]
            user: &'a Option<User>,
            token: &'a str,
            version: u8,
            #[serde(skip_serializing_if = "Option::is_none")]
            locale: &'a Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            guild_locale: &'a Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            message: &'a Option<Message>,
            #[serde(skip_serializing_if = "Option::is_none")]
            app_permissions: &'a Option<Permissions>,
            #[serde(skip_serializing_if = "Option::is_none")]
            attachment_size_limit: &'a Option<u64>,
        }

        let data = match self.data.as_ref() {
            Some(InteractionData::ApplicationCommand(data)) => {
                Some(serde_json::to_value(data).map_err(serde::ser::Error::custom)?)
            }
            Some(InteractionData::Autocomplete(data)) => {
                Some(serde_json::to_value(data).map_err(serde::ser::Error::custom)?)
            }
            Some(InteractionData::MessageComponent(data)) => {
                Some(serde_json::to_value(data).map_err(serde::ser::Error::custom)?)
            }
            Some(InteractionData::ModalSubmit(data)) => {
                Some(serde_json::to_value(data).map_err(serde::ser::Error::custom)?)
            }
            None => None,
        };

        RawInteraction {
            id: &self.id,
            application_id: &self.application_id,
            r#type: self.r#type,
            data,
            guild_id: &self.guild_id,
            channel_id: &self.channel_id,
            member: &self.member,
            user: &self.user,
            token: &self.token,
            version: self.version,
            locale: &self.locale,
            guild_locale: &self.guild_locale,
            message: &self.message,
            app_permissions: &self.app_permissions,
            attachment_size_limit: &self.attachment_size_limit,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Interaction {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct RawInteraction {
            id: InteractionId,
            application_id: ApplicationId,
            r#type: InteractionType,
            data: Option<Value>,
            guild_id: Option<GuildId>,
            channel_id: Option<ChannelId>,
            member: Option<Member>,
            user: Option<User>,
            token: String,
            version: u8,
            locale: Option<String>,
            guild_locale: Option<String>,
            message: Option<Message>,
            app_permissions: Option<Permissions>,
            attachment_size_limit: Option<u64>,
        }

        let raw = RawInteraction::deserialize(deserializer)?;
        let data = match (raw.r#type, raw.data) {
            (InteractionType::ApplicationCommand, Some(value)) => {
                Some(InteractionData::ApplicationCommand(Box::new(
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?,
                )))
            }
            (InteractionType::ApplicationCommandAutocomplete, Some(value)) => {
                Some(InteractionData::Autocomplete(Box::new(
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?,
                )))
            }
            (InteractionType::MessageComponent, Some(value)) => {
                Some(InteractionData::MessageComponent(Box::new(
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?,
                )))
            }
            (InteractionType::ModalSubmit, Some(value)) => Some(InteractionData::ModalSubmit(
                Box::new(serde_json::from_value(value).map_err(serde::de::Error::custom)?),
            )),
            // Pings carry no data; unexpected data elsewhere is ignored.
            _ => None,
        };

        Ok(Self {
            id: raw.id,
            application_id: raw.application_id,
            r#type: raw.r#type,
            data,
            guild_id: raw.guild_id,
            channel_id: raw.channel_id,
            member: raw.member,
            user: raw.user,
            token: raw.token,
            version: raw.version,
            locale: raw.locale,
            guild_locale: raw.guild_locale,
            message: raw.message,
            app_permissions: raw.app_permissions,
            attachment_size_limit: raw.attachment_size_limit,
        })
    }
}

/// The `data` of an application-command interaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandData {
    /// The command's ID.
    pub id: ApplicationCommandId,
    /// The command's name.
    pub name: String,
    /// The command type.
    pub r#type: Option<u8>,
    /// The resolved entities referenced by the invocation.
    pub resolved: Option<ResolvedData>,
    /// The provided options, in the order the user supplied them.
    #[serde(default)]
    pub options: Vec<CommandDataOption>,
    /// The guild the command is registered to, when guild-specific.
    pub guild_id: Option<GuildId>,
    /// The targeted user or message, for context-menu commands.
    pub target_id: Option<GenericId>,
}

impl CommandData {
    /// The command type, when it is a documented value.
    pub fn command_type(&self) -> Option<CommandType> {
        CommandType::from_value(self.r#type?)
    }
}

/// The type of an application command, from the Application Command Type
/// table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CommandType {
    /// A slash command (`CHAT_INPUT`).
    ChatInput = 1,
    /// A user context-menu command.
    User = 2,
    /// A message context-menu command.
    Message = 3,
    /// The application's Activity entry point.
    PrimaryEntryPoint = 4,
}

impl CommandType {
    /// Parse from the documented numeric value.
    pub const fn from_value(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::ChatInput),
            2 => Some(Self::User),
            3 => Some(Self::Message),
            4 => Some(Self::PrimaryEntryPoint),
            _ => None,
        }
    }

    /// The documented numeric value.
    pub const fn as_value(self) -> u8 {
        self as u8
    }
}

impl Serialize for CommandType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(u64::from(self.as_value()))
    }
}

impl<'de> Deserialize<'de> for CommandType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TypeVisitor;

        impl serde::de::Visitor<'_> for TypeVisitor {
            type Value = CommandType;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("a documented application command type value")
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                let value = u8::try_from(value).map_err(|_| {
                    serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(value), &self)
                })?;
                CommandType::from_value(value).ok_or_else(|| {
                    serde::de::Error::invalid_value(
                        serde::de::Unexpected::Unsigned(u64::from(value)),
                        &self,
                    )
                })
            }
        }

        deserializer.deserialize_u64(TypeVisitor)
    }
}

/// One option value in a command invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandDataOption {
    /// The option's name.
    pub name: String,
    /// The documented option type value.
    pub r#type: u8,
    /// The provided value; mutually exclusive with nested `options`.
    pub value: Option<Value>,
    /// Nested options, present for subcommands and subcommand groups.
    pub options: Option<Vec<CommandDataOption>>,
    /// Whether the option is focused for autocomplete.
    pub focused: Option<bool>,
}

/// Entities referenced by a command invocation, keyed by snowflake string.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ResolvedData {
    /// Resolved users.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub users: std::collections::HashMap<String, User>,
    /// Resolved members, for users present in the guild.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub members: std::collections::HashMap<String, Member>,
    /// Resolved roles.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub roles: std::collections::HashMap<String, Role>,
    /// Resolved attachments.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub attachments: std::collections::HashMap<String, crate::message::Attachment>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTERACTION: &str = r#"{
        "id": "1064160133216616528",
        "application_id": "80351110224678912",
        "type": 2,
        "data": {
            "id": "1064160133216616529",
            "name": "blep",
            "type": 1,
            "resolved": {
                "users": {
                    "80351110224678912": { "id": "80351110224678912", "username": "Nelly", "discriminator": "0" }
                }
            },
            "options": [
                { "name": "animal", "type": 3, "value": "cat" },
                { "name": "detail", "type": 1, "options": [ { "name": "x", "type": 3, "value": "y" } ] }
            ]
        },
        "guild_id": "290926798999268355",
        "channel_id": "417722298349095947",
        "member": {
            "user": { "id": "80351110224678912", "username": "Nelly", "discriminator": "0" },
            "roles": [],
            "deaf": false,
            "mute": false
        },
        "token": "a-secret-token",
        "version": 1,
        "locale": "en-US",
        "app_permissions": "446619818561"
    }"#;

    #[test]
    fn interaction_deserializes() {
        let interaction: Interaction = serde_json::from_str(INTERACTION).unwrap();
        assert_eq!(interaction.r#type, InteractionType::ApplicationCommand);
        assert_eq!(
            interaction.guild_id.map(|g| g.get()),
            Some(290_926_798_999_268_355)
        );
        assert!(interaction.member.is_some());
        assert_eq!(interaction.user, None);
        assert_eq!(interaction.version, 1);
    }

    #[test]
    fn command_data_decodes() {
        let interaction: Interaction = serde_json::from_str(INTERACTION).unwrap();
        let data = interaction.command_data().expect("command data decodes");
        assert_eq!(data.name, "blep");
        assert_eq!(data.command_type(), Some(CommandType::ChatInput));
        assert_eq!(data.options.len(), 2);
        assert_eq!(data.options[0].value.as_ref().unwrap(), "cat");
        // Subcommand nesting round-trips.
        assert_eq!(data.options[1].options.as_ref().unwrap().len(), 1);
        let resolved = data.resolved.as_ref().unwrap();
        assert_eq!(resolved.users.len(), 1);
        assert_eq!(resolved.users["80351110224678912"].username, "Nelly");
    }

    #[test]
    fn data_is_typed_per_interaction_kind() {
        let component: Interaction = serde_json::from_str(
            r#"{
                "id": "1", "application_id": "2", "type": 3,
                "data": { "custom_id": "button_1", "component_type": 2 },
                "token": "t", "version": 1
            }"#,
        )
        .unwrap();
        let Some(InteractionData::MessageComponent(data)) = component.data.as_ref() else {
            panic!("expected component data")
        };
        assert_eq!(data.custom_id, "button_1");
        assert_eq!(component.command_data(), None);

        let modal: Interaction = serde_json::from_str(
            r#"{
                "id": "1", "application_id": "2", "type": 5,
                "data": { "custom_id": "modal_1", "components": [] },
                "token": "t", "version": 1
            }"#,
        )
        .unwrap();
        let Some(InteractionData::ModalSubmit(data)) = modal.data.as_ref() else {
            panic!("expected modal data")
        };
        assert_eq!(data.custom_id, "modal_1");
    }

    #[test]
    fn autocomplete_data_decodes_as_command_data() {
        let interaction: Interaction = serde_json::from_str(
            r#"{
                "id": "1", "application_id": "2", "type": 4,
                "data": {
                    "id": "3", "name": "blep",
                    "options": [ { "name": "animal", "type": 3, "value": "c", "focused": true } ]
                },
                "token": "t", "version": 1
            }"#,
        )
        .unwrap();
        assert_eq!(
            interaction.r#type,
            InteractionType::ApplicationCommandAutocomplete
        );
        let data = interaction.command_data().unwrap();
        assert!(data.options[0].focused.unwrap_or(false));
    }

    #[test]
    fn interaction_type_values() {
        assert_eq!(
            InteractionType::from_value(4),
            Some(InteractionType::ApplicationCommandAutocomplete)
        );
        assert_eq!(InteractionType::from_value(6), None);
        assert_eq!(
            serde_json::to_string(&InteractionType::MessageComponent).unwrap(),
            "3"
        );
        // Documented callback type integers (note the gaps: 2, 3, 11).
        assert_eq!(
            InteractionCallbackType::from_value(4),
            Some(InteractionCallbackType::ChannelMessageWithSource)
        );
        assert_eq!(InteractionCallbackType::from_value(2), None);
        assert_eq!(InteractionCallbackType::LaunchActivity.as_value(), 12);
    }

    #[test]
    fn interaction_roundtrips() {
        let interaction: Interaction = serde_json::from_str(INTERACTION).unwrap();
        let json = serde_json::to_string(&interaction).unwrap();
        let reparsed: Interaction = serde_json::from_str(&json).unwrap();
        assert_eq!(reparsed, interaction);
    }
}
