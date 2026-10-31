//! The Discord audit log entry object.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_json::Value;

use axis_core::{ApplicationId, AuditLogEntryId, ChannelId, GenericId, MessageId, UserId};

/// An entry in a guild's audit log.
///
/// Entries reference affected entities by ID; the accompanying object
/// lists (users, webhooks, integrations, ...) from the audit-log response
/// provide the lookups.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// The entry ID.
    pub id: AuditLogEntryId,
    /// What kind of action occurred.
    pub action_type: AuditLogAction,
    /// The affected entity's ID — a user, role, channel, webhook, or
    /// another object depending on the action.
    pub target_id: Option<GenericId>,
    /// The user or application that made the changes.
    pub user_id: Option<UserId>,
    /// The changes made to the target.
    pub changes: Option<Vec<AuditLogChange>>,
    /// Extra information, for the action types that provide it.
    pub options: Option<AuditEntryInfo>,
    /// The reason for the change, when the acting user supplied one.
    pub reason: Option<String>,
}

/// One property change inside an audit log entry.
///
/// `new_value` absent while `old_value` present means the property was
/// reset to null, and vice versa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditLogChange {
    /// The changed property's name.
    pub key: String,
    /// The value after the change.
    pub new_value: Option<Value>,
    /// The value before the change.
    pub old_value: Option<Value>,
}

/// Extra information attached to certain audit log entry types.
///
/// Fields are decimal strings for some keys, snowflakes for others,
/// exactly as documented.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntryInfo {
    /// The application whose command permissions changed.
    pub application_id: Option<ApplicationId>,
    /// The auto-moderation rule that triggered, by name.
    pub auto_moderation_rule_name: Option<String>,
    /// The auto-moderation rule's trigger type.
    pub auto_moderation_rule_trigger_type: Option<String>,
    /// The affected channel.
    pub channel_id: Option<ChannelId>,
    /// How many of something the action affected (messages deleted,
    /// members moved, ...).
    pub count: Option<String>,
    /// How many days of messages were deleted, for member prunes.
    pub delete_member_days: Option<String>,
    /// The overwrite target's ID, for channel overwrite events.
    pub id: Option<GenericId>,
    /// How many members were removed, for member prunes.
    pub members_removed: Option<String>,
    /// The pinned or unpinned message.
    pub message_id: Option<MessageId>,
    /// The role name, for channel overwrite events targeting a role.
    pub role_name: Option<String>,
    /// The overwrite target kind: `"0"` for roles, `"1"` for members.
    pub r#type: Option<String>,
    /// The integration type, for member kicks and role updates.
    pub integration_type: Option<String>,
    /// The voice channel status, for status events.
    pub status: Option<String>,
}

/// The documented audit log action types, with a catch-all for values
/// Discord adds later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuditLogAction {
    /// 1: a guild was updated.
    GuildUpdate,
    /// 10: a channel was created.
    ChannelCreate,
    /// 11: a channel was updated.
    ChannelUpdate,
    /// 12: a channel was deleted.
    ChannelDelete,
    /// 13: a channel permission overwrite was created.
    ChannelOverwriteCreate,
    /// 14: a channel permission overwrite was updated.
    ChannelOverwriteUpdate,
    /// 15: a channel permission overwrite was deleted.
    ChannelOverwriteDelete,
    /// 16: a channel's position changed.
    ChannelPositionUpdate,
    /// 20: a member was kicked.
    MemberKick,
    /// 21: members were pruned.
    MemberPrune,
    /// 22: a member was banned.
    MemberBanAdd,
    /// 23: a ban was lifted.
    MemberBanRemove,
    /// 24: a member was updated.
    MemberUpdate,
    /// 25: a member's roles changed.
    MemberRoleUpdate,
    /// 26: a member was moved between voice channels.
    MemberMove,
    /// 27: a member was disconnected from voice.
    MemberDisconnect,
    /// 28: a bot was added.
    BotAdd,
    /// 30: a role was created.
    RoleCreate,
    /// 31: a role was updated.
    RoleUpdate,
    /// 32: a role was deleted.
    RoleDelete,
    /// 33: a role's position changed.
    RolePositionUpdate,
    /// 40: an invite was created.
    InviteCreate,
    /// 41: an invite was updated.
    InviteUpdate,
    /// 42: an invite was deleted.
    InviteDelete,
    /// 50: a webhook was created.
    WebhookCreate,
    /// 51: a webhook was updated.
    WebhookUpdate,
    /// 52: a webhook was deleted.
    WebhookDelete,
    /// 60: an emoji was created.
    EmojiCreate,
    /// 61: an emoji was updated.
    EmojiUpdate,
    /// 62: an emoji was deleted.
    EmojiDelete,
    /// 72: a message was deleted.
    MessageDelete,
    /// 73: messages were bulk deleted.
    MessageBulkDelete,
    /// 74: a message was pinned.
    MessagePin,
    /// 75: a message was unpinned.
    MessageUnpin,
    /// 80: an integration was created.
    IntegrationCreate,
    /// 81: an integration was updated.
    IntegrationUpdate,
    /// 82: an integration was deleted.
    IntegrationDelete,
    /// 83: a stage instance was created.
    StageInstanceCreate,
    /// 84: a stage instance was updated.
    StageInstanceUpdate,
    /// 85: a stage instance was deleted.
    StageInstanceDelete,
    /// 90: a sticker was created.
    StickerCreate,
    /// 91: a sticker was updated.
    StickerUpdate,
    /// 92: a sticker was deleted.
    StickerDelete,
    /// 100: a scheduled event was created.
    GuildScheduledEventCreate,
    /// 101: a scheduled event was updated.
    GuildScheduledEventUpdate,
    /// 102: a scheduled event was deleted.
    GuildScheduledEventDelete,
    /// 110: a thread was created.
    ThreadCreate,
    /// 111: a thread was updated.
    ThreadUpdate,
    /// 112: a thread was deleted.
    ThreadDelete,
    /// 121: command permissions were updated.
    ApplicationCommandPermissionUpdate,
    /// 130: a soundboard sound was created.
    SoundboardSoundCreate,
    /// 131: a soundboard sound was updated.
    SoundboardSoundUpdate,
    /// 132: a soundboard sound was deleted.
    SoundboardSoundDelete,
    /// 140: an auto-moderation rule was created.
    AutoModerationRuleCreate,
    /// 141: an auto-moderation rule was updated.
    AutoModerationRuleUpdate,
    /// 142: an auto-moderation rule was deleted.
    AutoModerationRuleDelete,
    /// 143: auto-moderation blocked a message.
    AutoModerationBlockMessage,
    /// 144: auto-moderation flagged a message to a channel.
    AutoModerationFlagToChannel,
    /// 145: auto-moderation timed a user out.
    AutoModerationUserCommunicationDisabled,
    /// 146: auto-moderation quarantined a user.
    AutoModerationQuarantineUser,
    /// 150: a creator monetization request was created.
    CreatorMonetizationRequestCreated,
    /// 151: creator monetization terms were accepted.
    CreatorMonetizationTermsAccepted,
    /// 163: an onboarding prompt was created.
    OnboardingPromptCreate,
    /// 164: an onboarding prompt was updated.
    OnboardingPromptUpdate,
    /// 165: an onboarding prompt was deleted.
    OnboardingPromptDelete,
    /// 166: onboarding was created.
    OnboardingCreate,
    /// 167: onboarding was updated.
    OnboardingUpdate,
    /// 190: home settings were created.
    HomeSettingsCreate,
    /// 191: home settings were updated.
    HomeSettingsUpdate,
    /// 192: a voice channel status was created.
    VoiceChannelStatusCreate,
    /// 193: a voice channel status was deleted.
    VoiceChannelStatusDelete,
    /// A value not in the documented table (Discord may add actions).
    Unknown(u16),
}

impl AuditLogAction {
    /// Parse a documented numeric action value.
    pub const fn from_value(value: u16) -> Self {
        match value {
            1 => Self::GuildUpdate,
            10 => Self::ChannelCreate,
            11 => Self::ChannelUpdate,
            12 => Self::ChannelDelete,
            13 => Self::ChannelOverwriteCreate,
            14 => Self::ChannelOverwriteUpdate,
            15 => Self::ChannelOverwriteDelete,
            16 => Self::ChannelPositionUpdate,
            20 => Self::MemberKick,
            21 => Self::MemberPrune,
            22 => Self::MemberBanAdd,
            23 => Self::MemberBanRemove,
            24 => Self::MemberUpdate,
            25 => Self::MemberRoleUpdate,
            26 => Self::MemberMove,
            27 => Self::MemberDisconnect,
            28 => Self::BotAdd,
            30 => Self::RoleCreate,
            31 => Self::RoleUpdate,
            32 => Self::RoleDelete,
            33 => Self::RolePositionUpdate,
            40 => Self::InviteCreate,
            41 => Self::InviteUpdate,
            42 => Self::InviteDelete,
            50 => Self::WebhookCreate,
            51 => Self::WebhookUpdate,
            52 => Self::WebhookDelete,
            60 => Self::EmojiCreate,
            61 => Self::EmojiUpdate,
            62 => Self::EmojiDelete,
            72 => Self::MessageDelete,
            73 => Self::MessageBulkDelete,
            74 => Self::MessagePin,
            75 => Self::MessageUnpin,
            80 => Self::IntegrationCreate,
            81 => Self::IntegrationUpdate,
            82 => Self::IntegrationDelete,
            83 => Self::StageInstanceCreate,
            84 => Self::StageInstanceUpdate,
            85 => Self::StageInstanceDelete,
            90 => Self::StickerCreate,
            91 => Self::StickerUpdate,
            92 => Self::StickerDelete,
            100 => Self::GuildScheduledEventCreate,
            101 => Self::GuildScheduledEventUpdate,
            102 => Self::GuildScheduledEventDelete,
            110 => Self::ThreadCreate,
            111 => Self::ThreadUpdate,
            112 => Self::ThreadDelete,
            121 => Self::ApplicationCommandPermissionUpdate,
            130 => Self::SoundboardSoundCreate,
            131 => Self::SoundboardSoundUpdate,
            132 => Self::SoundboardSoundDelete,
            140 => Self::AutoModerationRuleCreate,
            141 => Self::AutoModerationRuleUpdate,
            142 => Self::AutoModerationRuleDelete,
            143 => Self::AutoModerationBlockMessage,
            144 => Self::AutoModerationFlagToChannel,
            145 => Self::AutoModerationUserCommunicationDisabled,
            146 => Self::AutoModerationQuarantineUser,
            150 => Self::CreatorMonetizationRequestCreated,
            151 => Self::CreatorMonetizationTermsAccepted,
            163 => Self::OnboardingPromptCreate,
            164 => Self::OnboardingPromptUpdate,
            165 => Self::OnboardingPromptDelete,
            166 => Self::OnboardingCreate,
            167 => Self::OnboardingUpdate,
            190 => Self::HomeSettingsCreate,
            191 => Self::HomeSettingsUpdate,
            192 => Self::VoiceChannelStatusCreate,
            193 => Self::VoiceChannelStatusDelete,
            _ => Self::Unknown(value),
        }
    }

    /// The documented numeric value.
    pub const fn as_value(self) -> u16 {
        match self {
            Self::GuildUpdate => 1,
            Self::ChannelCreate => 10,
            Self::ChannelUpdate => 11,
            Self::ChannelDelete => 12,
            Self::ChannelOverwriteCreate => 13,
            Self::ChannelOverwriteUpdate => 14,
            Self::ChannelOverwriteDelete => 15,
            Self::ChannelPositionUpdate => 16,
            Self::MemberKick => 20,
            Self::MemberPrune => 21,
            Self::MemberBanAdd => 22,
            Self::MemberBanRemove => 23,
            Self::MemberUpdate => 24,
            Self::MemberRoleUpdate => 25,
            Self::MemberMove => 26,
            Self::MemberDisconnect => 27,
            Self::BotAdd => 28,
            Self::RoleCreate => 30,
            Self::RoleUpdate => 31,
            Self::RoleDelete => 32,
            Self::RolePositionUpdate => 33,
            Self::InviteCreate => 40,
            Self::InviteUpdate => 41,
            Self::InviteDelete => 42,
            Self::WebhookCreate => 50,
            Self::WebhookUpdate => 51,
            Self::WebhookDelete => 52,
            Self::EmojiCreate => 60,
            Self::EmojiUpdate => 61,
            Self::EmojiDelete => 62,
            Self::MessageDelete => 72,
            Self::MessageBulkDelete => 73,
            Self::MessagePin => 74,
            Self::MessageUnpin => 75,
            Self::IntegrationCreate => 80,
            Self::IntegrationUpdate => 81,
            Self::IntegrationDelete => 82,
            Self::StageInstanceCreate => 83,
            Self::StageInstanceUpdate => 84,
            Self::StageInstanceDelete => 85,
            Self::StickerCreate => 90,
            Self::StickerUpdate => 91,
            Self::StickerDelete => 92,
            Self::GuildScheduledEventCreate => 100,
            Self::GuildScheduledEventUpdate => 101,
            Self::GuildScheduledEventDelete => 102,
            Self::ThreadCreate => 110,
            Self::ThreadUpdate => 111,
            Self::ThreadDelete => 112,
            Self::ApplicationCommandPermissionUpdate => 121,
            Self::SoundboardSoundCreate => 130,
            Self::SoundboardSoundUpdate => 131,
            Self::SoundboardSoundDelete => 132,
            Self::AutoModerationRuleCreate => 140,
            Self::AutoModerationRuleUpdate => 141,
            Self::AutoModerationRuleDelete => 142,
            Self::AutoModerationBlockMessage => 143,
            Self::AutoModerationFlagToChannel => 144,
            Self::AutoModerationUserCommunicationDisabled => 145,
            Self::AutoModerationQuarantineUser => 146,
            Self::CreatorMonetizationRequestCreated => 150,
            Self::CreatorMonetizationTermsAccepted => 151,
            Self::OnboardingPromptCreate => 163,
            Self::OnboardingPromptUpdate => 164,
            Self::OnboardingPromptDelete => 165,
            Self::OnboardingCreate => 166,
            Self::OnboardingUpdate => 167,
            Self::HomeSettingsCreate => 190,
            Self::HomeSettingsUpdate => 191,
            Self::VoiceChannelStatusCreate => 192,
            Self::VoiceChannelStatusDelete => 193,
            Self::Unknown(value) => value,
        }
    }
}

impl Serialize for AuditLogAction {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(u64::from(self.as_value()))
    }
}

impl<'de> Deserialize<'de> for AuditLogAction {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ActionVisitor;

        impl de::Visitor<'_> for ActionVisitor {
            type Value = AuditLogAction;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("a documented audit log action value (u16)")
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                let value = u16::try_from(value).map_err(|_| {
                    de::Error::invalid_value(
                        de::Unexpected::Unsigned(value),
                        &"an audit log action value in range",
                    )
                })?;
                // Lenient: unknown actions become `Unknown`, mirroring the
                // catch-all in the enum itself.
                Ok(AuditLogAction::from_value(value))
            }
        }

        deserializer.deserialize_u64(ActionVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENTRY: &str = r#"{
        "id": "1034411239858913690",
        "action_type": 22,
        "target_id": "80351110224678912",
        "user_id": "87616711559942144",
        "changes": [
            { "key": "username", "old_value": "old-name", "new_value": "new-name" }
        ],
        "reason": "spamming in #general"
    }"#;

    #[test]
    fn entry_deserializes() {
        let entry: AuditLogEntry = serde_json::from_str(ENTRY).unwrap();
        assert_eq!(entry.id.get(), 1_034_411_239_858_913_690);
        assert_eq!(entry.action_type, AuditLogAction::MemberBanAdd);
        assert_eq!(
            entry.target_id.map(|id| id.get()),
            Some(80_351_110_224_678_912)
        );
        assert_eq!(
            entry.user_id.map(|id| id.get()),
            Some(87_616_711_559_942_144)
        );
        assert_eq!(entry.reason.as_deref(), Some("spamming in #general"));
        let changes = entry.changes.unwrap();
        assert_eq!(changes[0].key, "username");
        assert_eq!(changes[0].new_value.as_ref().unwrap(), "new-name");
    }

    #[test]
    fn optional_entry_info_deserializes() {
        // Shape from a MEMBER_PRUNE entry.
        let entry: AuditLogEntry = serde_json::from_str(
            r#"{
                "id": "1",
                "action_type": 21,
                "options": {
                    "delete_member_days": "7",
                    "members_removed": "3"
                }
            }"#,
        )
        .unwrap();
        let options = entry.options.unwrap();
        assert_eq!(options.delete_member_days.as_deref(), Some("7"));
        assert_eq!(options.members_removed.as_deref(), Some("3"));
    }

    #[test]
    fn action_values_roundtrip() {
        assert_eq!(AuditLogAction::from_value(1), AuditLogAction::GuildUpdate);
        assert_eq!(
            AuditLogAction::from_value(400),
            AuditLogAction::Unknown(400)
        );
        assert_eq!(AuditLogAction::Unknown(400).as_value(), 400);
        assert_eq!(
            AuditLogAction::from_value(193),
            AuditLogAction::VoiceChannelStatusDelete
        );
        // Serde: numbers in, numbers out (unknown values preserved).
        let json = serde_json::to_string(&AuditLogAction::AutoModerationBlockMessage).unwrap();
        assert_eq!(json, "143");
        let parsed: AuditLogAction = serde_json::from_str("999").unwrap();
        assert_eq!(parsed, AuditLogAction::Unknown(999));
    }

    #[test]
    fn entry_roundtrips() {
        let entry: AuditLogEntry = serde_json::from_str(ENTRY).unwrap();
        let json = serde_json::to_string(&entry).unwrap();
        let reparsed: AuditLogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(reparsed, entry);
    }
}
