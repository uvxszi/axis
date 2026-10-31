//! Application command models: registered commands, their options, and
//! per-guild command permissions.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use axis_core::{ApplicationCommandId, ApplicationId, GuildId, Permissions};

use crate::channel::ChannelType;
use crate::interaction::CommandType;

/// A registered application command, as returned by the command
/// registration endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationCommand {
    /// The command's ID.
    pub id: ApplicationCommandId,
    /// The command type; defaults to [`CommandType::ChatInput`] when
    /// absent.
    pub r#type: Option<CommandType>,
    /// The application that owns the command.
    pub application_id: ApplicationId,
    /// The guild the command is registered in, when guild-scoped.
    pub guild_id: Option<GuildId>,
    /// The command name (1-32 characters).
    pub name: String,
    /// The command description (1-100 characters for chat-input commands).
    pub description: String,
    /// Name translations, keyed by locale.
    pub name_localizations: Option<HashMap<String, String>>,
    /// Description translations, keyed by locale.
    pub description_localizations: Option<HashMap<String, String>>,
    /// The command's options (chat-input commands only; up to 25).
    #[serde(default)]
    pub options: Vec<ApplicationCommandOption>,
    /// The default permission members need to use the command. `None`
    /// (JSON null) means "enabled for everyone by default".
    pub default_member_permissions: Option<Permissions>,
    /// Whether the command can be used in DMs (global commands only).
    /// Deprecated by Discord in favor of `contexts`.
    pub dm_permission: Option<bool>,
    /// Whether the command is age-restricted.
    pub nsfw: Option<bool>,
    /// The auto-incrementing revision of the command definition.
    pub version: Option<ApplicationCommandId>,
}

/// One option (parameter) of an application command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationCommandOption {
    /// The option type.
    pub r#type: ApplicationCommandOptionType,
    /// The option name (1-32 characters).
    pub name: String,
    /// The option description (1-100 characters).
    pub description: String,
    /// Whether the option must be provided. Required options must be
    /// listed before optional ones.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    /// Fixed choices the user may pick (string/integer/number options
    /// only; mutually exclusive with `autocomplete`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<ApplicationCommandOptionChoice>>,
    /// Nested options, for subcommands and subcommand groups.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<ApplicationCommandOption>>,
    /// Channel kinds accepted by a channel option.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_types: Option<Vec<ChannelType>>,
    /// Whether the option supports autocomplete (string/integer/number
    /// only; mutually exclusive with `choices`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocomplete: Option<bool>,
    /// The smallest accepted value (integer/number options).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f64>,
    /// The largest accepted value (integer/number options).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,
    /// The shortest accepted input (string options).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u16>,
    /// The longest accepted input (string options; at most 6000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u16>,
}

api_enum! {
    /// The type of an application command option, from the documented
    /// Application Command Option Type table.
    ApplicationCommandOptionType: u8 {
        /// A nested subcommand.
        SubCommand = 1,
        /// A group of subcommands.
        SubCommandGroup = 2,
        /// A free-form string argument.
        String = 3,
        /// An integer argument (±2^53-1).
        Integer = 4,
        /// A true/false argument.
        Boolean = 5,
        /// A user argument.
        User = 6,
        /// A channel argument.
        Channel = 7,
        /// A role argument.
        Role = 8,
        /// A user or role argument.
        Mentionable = 9,
        /// A floating-point argument.
        Number = 10,
        /// An uploaded file argument.
        Attachment = 11,
    }
}

/// A fixed choice for a string/integer/number option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationCommandOptionChoice {
    /// The choice name shown to the user (1-100 characters).
    pub name: String,
    /// Name translations, keyed by locale.
    pub name_localizations: Option<HashMap<String, String>>,
    /// The value sent to the handler (string, integer, or number).
    pub value: serde_json::Value,
}

/// The per-guild permissions of application commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildApplicationCommandPermissions {
    /// The command the permissions apply to, or the application ID when
    /// they apply to all commands without explicit overwrites.
    pub id: ApplicationCommandId,
    /// The application that owns the command.
    pub application_id: ApplicationId,
    /// The guild the permissions are scoped to.
    pub guild_id: GuildId,
    /// The permission entries (up to 100).
    pub permissions: Vec<ApplicationCommandPermission>,
}

/// One permission entry for an application command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationCommandPermission {
    /// The ID of the role, user, or channel this entry applies to.
    pub id: axis_core::GenericId,
    /// Whether `id` names a role, a user, or a channel.
    pub r#type: ApplicationCommandPermissionType,
    /// Whether to allow (`true`) or disallow (`false`) the command.
    pub permission: bool,
}

api_enum! {
    /// The target kind of an application command permission entry.
    ApplicationCommandPermissionType: u8 {
        /// The entry targets a role.
        Role = 1,
        /// The entry targets a user.
        User = 2,
        /// The entry targets a channel.
        Channel = 3,
    }
}

/// The request body fields for creating, editing, or bulk-overwriting
/// application commands.
///
/// For creation, `name` and `description` are required; for edits every
/// provided field fully replaces the stored one and omitted fields are
/// left unchanged.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ApplicationCommandFields {
    /// The command name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The command description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The command type (defaults to chat input).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<CommandType>,
    /// The command options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<ApplicationCommandOption>>,
    /// The default member permissions required to use the command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_member_permissions: Option<Permissions>,
    /// Whether the command may be used in DMs (global commands only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dm_permission: Option<bool>,
    /// Whether the command is age-restricted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMAND: &str = r#"{
        "id": "1064160133216616529",
        "type": 1,
        "application_id": "80351110224678912",
        "name": "blep",
        "description": "Pet a virtual animal",
        "options": [
            {
                "type": 3,
                "name": "animal",
                "description": "The animal to pet",
                "required": true,
                "choices": [ { "name": "Dog", "value": "animal_dog" } ]
            },
            {
                "type": 7,
                "name": "channel",
                "description": "Where to pet",
                "channel_types": [0, 5]
            }
        ],
        "default_member_permissions": "8",
        "dm_permission": false,
        "nsfw": false,
        "version": "1064160133216616530"
    }"#;

    #[test]
    fn application_command_deserializes() {
        let command: ApplicationCommand = serde_json::from_str(COMMAND).unwrap();
        assert_eq!(command.id.get(), 1_064_160_133_216_616_529);
        assert_eq!(command.r#type, Some(CommandType::ChatInput));
        assert_eq!(command.name, "blep");
        assert_eq!(command.options.len(), 2);
        assert_eq!(
            command.options[0].r#type,
            ApplicationCommandOptionType::String
        );
        let choices = command.options[0].choices.as_ref().unwrap();
        assert_eq!(choices[0].value, serde_json::json!("animal_dog"));
        assert_eq!(
            command.options[1].channel_types.as_ref().unwrap(),
            &[ChannelType::GuildText, ChannelType::GuildAnnouncement]
        );
        // default_member_permissions "8" == ADMINISTRATOR.
        assert_eq!(
            command.default_member_permissions,
            Some(Permissions::ADMINISTRATOR)
        );
        assert_eq!(command.dm_permission, Some(false));
    }

    #[test]
    fn fields_serialize_only_set_values() {
        fn string_option() -> ApplicationCommandOption {
            ApplicationCommandOption {
                r#type: ApplicationCommandOptionType::String,
                name: "text".to_owned(),
                description: "The text".to_owned(),
                required: Some(true),
                choices: None,
                options: None,
                channel_types: None,
                autocomplete: None,
                min_value: None,
                max_value: None,
                min_length: None,
                max_length: None,
            }
        }

        let fields = ApplicationCommandFields {
            name: Some("echo".to_owned()),
            description: Some("Repeats text".to_owned()),
            options: Some(vec![string_option()]),
            ..ApplicationCommandFields::default()
        };
        let json = serde_json::to_value(&fields).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "name": "echo",
                "description": "Repeats text",
                "options": [ { "type": 3, "name": "text", "description": "The text", "required": true } ]
            })
        );
    }

    #[test]
    fn permissions_payloads() {
        let perms: GuildApplicationCommandPermissions = serde_json::from_str(
            r#"{
                "id": "1064160133216616529",
                "application_id": "80351110224678912",
                "guild_id": "290926798999268355",
                "permissions": [
                    { "id": "41771983423143937", "type": 1, "permission": true },
                    { "id": "80351110224678912", "type": 2, "permission": false }
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(perms.permissions.len(), 2);
        assert_eq!(
            perms.permissions[0].r#type,
            ApplicationCommandPermissionType::Role
        );
        assert!(perms.permissions[0].permission);
        assert!(!perms.permissions[1].permission);

        // Serialization for the PUT endpoint.
        let json = serde_json::to_string(&perms.permissions).unwrap();
        assert!(json.contains(r#"{"id":41771983423143937,"type":1,"permission":true}"#));
    }

    #[test]
    fn option_type_values() {
        assert_eq!(
            ApplicationCommandOptionType::from_value(11),
            Some(ApplicationCommandOptionType::Attachment)
        );
        assert_eq!(ApplicationCommandOptionType::from_value(12), None);
        assert_eq!(
            serde_json::to_string(&ApplicationCommandOptionType::SubCommandGroup).unwrap(),
            "2"
        );
    }
}
