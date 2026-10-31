//! The Discord interaction object.
//!
//! The `Interaction` envelope is typed here; the command-specific `data`
//! is kept raw on the struct (its shape differs per interaction type) and
//! can be decoded with [`Interaction::command_data`] or — from Phase 5 —
//! through the command framework's typed views.

use serde::{Deserialize, Serialize};
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

/// The command type of an application command, as carried in interaction
/// data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CommandType {
    /// A slash command (`CHAT_INPUT`).
    ChatInput = 1,
    /// A user context-menu command.
    User = 2,
    /// A message context-menu command.
    Message = 3,
}

impl CommandType {
    /// Parse from the documented numeric value.
    pub const fn from_value(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::ChatInput),
            2 => Some(Self::User),
            3 => Some(Self::Message),
            _ => None,
        }
    }

    /// The documented numeric value.
    pub const fn as_value(self) -> u8 {
        self as u8
    }
}

/// An interaction: a command invocation, component press, autocomplete
/// request, or modal submission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interaction {
    /// The interaction ID.
    pub id: InteractionId,
    /// The application the interaction is for.
    pub application_id: ApplicationId,
    /// The interaction type.
    pub r#type: InteractionType,
    /// The raw interaction data — its shape depends on [`Self::r#type`].
    /// Decode command invocations with [`Self::command_data`].
    pub data: Option<Value>,
    /// The guild it was invoked in, when applicable.
    pub guild_id: Option<GuildId>,
    /// The channel it was invoked in, when applicable.
    pub channel_id: Option<ChannelId>,
    /// The invoking member, when invoked in a guild.
    pub member: Option<Member>,
    /// The invoking user, when invoked in a DM.
    pub user: Option<User>,
    /// The continuation token for responding (valid 15 minutes).
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
    /// Decode the interaction data as an application-command invocation.
    ///
    /// Returns `None` when there is no data or the data does not match the
    /// command shape (components and modals carry different payloads).
    pub fn command_data(&self) -> Option<CommandData> {
        serde_json::from_value(self.data.clone()?).ok()
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
        let resolved = data.resolved.unwrap();
        assert_eq!(resolved.users.len(), 1);
        assert_eq!(resolved.users["80351110224678912"].username, "Nelly");
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
    }
}
