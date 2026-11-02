//! The Discord channel object, channel types, and permission overwrites.

use serde::{Deserialize, Deserializer, Serialize};

use axis_core::{ChannelId, GenericId, GuildId, MessageId, Permissions, Timestamp, UserId};

use crate::user::User;

api_enum! {
    /// The type of a channel.
    ///
    /// Values from the official Channel Type table. Thread, forum, and
    /// media channels are represented, but their type-specific fields are
    /// not modeled yet.
    ChannelType: u8 {
        /// Text channel within a guild.
        GuildText = 0,
        /// Direct message between two users.
        Dm = 1,
        /// Voice channel within a guild.
        GuildVoice = 2,
        /// Direct message between multiple users.
        GroupDm = 3,
        /// Organizational category containing channels.
        GuildCategory = 4,
        /// Announcement channel (formerly news) within a guild.
        GuildAnnouncement = 5,
        /// Announcement thread.
        AnnouncementThread = 10,
        /// Public thread.
        PublicThread = 11,
        /// Private thread.
        PrivateThread = 12,
        /// Stage channel within a guild.
        GuildStageVoice = 13,
        /// Directory channel of a hub.
        GuildDirectory = 14,
        /// Forum channel.
        GuildForum = 15,
        /// Media channel.
        GuildMedia = 16,
    }
}

api_enum! {
    /// The target kind of a permission overwrite.
    OverwriteType: u8 {
        /// The overwrite targets a role.
        Role = 0,
        /// The overwrite targets a guild member.
        Member = 1,
    }
}

api_enum! {
    /// Video quality mode of a voice channel.
    VideoQualityMode: u8 {
        /// Discord chooses the quality (default).
        Auto = 1,
        /// Full quality (720p).
        Full = 2,
    }
}

/// A channel: guild text/voice/category/announcement/stage/forum channels,
/// or DM channels.
///
/// Common fields are always available; type-specific fields (voice, DM,
/// thread, forum) are optional and only populated in the contexts Discord
/// documents. Thread- and forum-specific fields are not modeled yet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    /// The channel ID.
    pub id: ChannelId,
    /// The channel type.
    pub r#type: ChannelType,
    /// The guild this channel belongs to; absent for DMs.
    pub guild_id: Option<GuildId>,
    /// Sorting position of the channel.
    pub position: Option<i32>,
    /// Explicit permission overwrites for members and roles.
    pub permission_overwrites: Option<Vec<PermissionOverwrite>>,
    /// The channel name (1–100 characters); absent for DMs.
    pub name: Option<String>,
    /// The channel topic (0–1024 characters).
    pub topic: Option<String>,
    /// Whether the channel is age-restricted.
    pub nsfw: Option<bool>,
    /// The ID of the last message sent in the channel (not updated for
    /// pins).
    pub last_message_id: Option<MessageId>,
    /// The bitrate (in bits/s) of a voice channel.
    pub bitrate: Option<u32>,
    /// The user limit of a voice channel.
    pub user_limit: Option<u32>,
    /// Seconds a user has to wait between sending messages (slowmode).
    pub rate_limit_per_user: Option<u32>,
    /// The recipients of a DM or group DM.
    pub recipients: Option<Vec<User>>,
    /// For group DMs, the creator; for threads, the thread creator.
    pub owner_id: Option<UserId>,
    /// The parent category (or, for threads, the parent channel) ID.
    pub parent_id: Option<ChannelId>,
    /// When the last pinned message was pinned.
    pub last_pin_timestamp: Option<Timestamp>,
    /// The RTC region ID for a voice channel (automatic when absent).
    pub rtc_region: Option<String>,
    /// The camera video quality mode of a voice channel.
    pub video_quality_mode: Option<VideoQualityMode>,
    /// Thread-specific state (archived, locked, ...); threads only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_metadata: Option<ThreadMetadata>,
    /// Approximate number of messages in a thread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_count: Option<u32>,
    /// Approximate number of members in a thread (stops counting at 50).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_count: Option<u32>,
    /// Number of messages ever sent in a thread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_message_sent: Option<u32>,
    /// Default auto-archive duration (minutes) for new threads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_auto_archive_duration: Option<u32>,
    /// Tags that can be applied to posts in a forum or media channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available_tags: Option<Vec<ForumTag>>,
    /// Tag IDs applied to a forum post (thread).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_tags: Option<Vec<GenericId>>,
    /// Channel flags bitfield (e.g. pinned forum post, require tag).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
}

impl Channel {
    /// Whether this channel is a thread (public, private, or announcement).
    pub fn is_thread(&self) -> bool {
        matches!(
            self.r#type,
            ChannelType::PublicThread
                | ChannelType::PrivateThread
                | ChannelType::AnnouncementThread
        )
    }

    /// Whether messages can be sent directly in this channel (text-like
    /// channels and threads; not categories, forums, or voice-only).
    pub fn is_text_based(&self) -> bool {
        matches!(
            self.r#type,
            ChannelType::GuildText
                | ChannelType::Dm
                | ChannelType::GroupDm
                | ChannelType::GuildAnnouncement
                | ChannelType::PublicThread
                | ChannelType::PrivateThread
                | ChannelType::AnnouncementThread
                | ChannelType::GuildVoice
                | ChannelType::GuildStageVoice
        )
    }

    /// Whether this is a voice or stage channel.
    pub fn is_voice(&self) -> bool {
        matches!(
            self.r#type,
            ChannelType::GuildVoice | ChannelType::GuildStageVoice
        )
    }

    /// Whether this is a forum or media channel (posts are threads).
    pub fn is_forum(&self) -> bool {
        matches!(
            self.r#type,
            ChannelType::GuildForum | ChannelType::GuildMedia
        )
    }
}

/// Thread-specific channel state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadMetadata {
    /// Whether the thread is archived.
    pub archived: bool,
    /// Minutes of inactivity before the thread auto-archives.
    pub auto_archive_duration: u32,
    /// When the archive status last changed.
    pub archive_timestamp: Option<Timestamp>,
    /// Whether the thread is locked (only moderators can unarchive).
    #[serde(default)]
    pub locked: bool,
    /// Whether non-moderators can add others to a private thread.
    pub invitable: Option<bool>,
    /// When the thread was created (threads created after 2022-01-09).
    pub create_timestamp: Option<Timestamp>,
}

/// A tag that can be applied to forum posts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForumTag {
    /// The tag ID.
    pub id: GenericId,
    /// The tag name (0–20 characters).
    pub name: String,
    /// Whether only moderators can apply the tag.
    #[serde(default)]
    pub moderated: bool,
    /// A custom emoji ID for the tag.
    pub emoji_id: Option<GenericId>,
    /// A unicode emoji for the tag.
    pub emoji_name: Option<String>,
}

/// A user's membership of a thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadMember {
    /// The thread ID (omitted in some gateway payloads).
    pub id: Option<ChannelId>,
    /// The user ID (omitted in some gateway payloads).
    pub user_id: Option<UserId>,
    /// When the user last joined the thread.
    pub join_timestamp: Option<Timestamp>,
    /// Notification settings flags.
    #[serde(default)]
    pub flags: u64,
}

/// A guild invite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Invite {
    /// The invite code (`discord.gg/<code>`).
    pub code: String,
    /// When the invite expires, if it does.
    pub expires_at: Option<Timestamp>,
    /// Maximum number of uses (0 = unlimited).
    pub max_uses: Option<u32>,
}

impl Invite {
    /// The shareable link.
    pub fn url(&self) -> String {
        format!("https://discord.gg/{}", self.code)
    }
}

/// The response of `GET /guilds/{id}/threads/active`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveThreads {
    /// The active threads.
    pub threads: Vec<Channel>,
    /// The bot's memberships of those threads.
    #[serde(default)]
    pub members: Vec<ThreadMember>,
}

/// An explicit permission overwrite for a role or member in a channel.
///
/// `allow` and `deny` default to no permissions when Discord omits or
/// nulls them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionOverwrite {
    /// The role or member this overwrite targets — see [`Self::r#type`].
    pub id: GenericId,
    /// Whether the target is a role or a member.
    pub r#type: OverwriteType,
    /// Permissions explicitly allowed by this overwrite.
    #[serde(default, deserialize_with = "deserialize_nullable_permissions")]
    pub allow: Permissions,
    /// Permissions explicitly denied by this overwrite.
    #[serde(default, deserialize_with = "deserialize_nullable_permissions")]
    pub deny: Permissions,
}

/// Discord sends overwrite `allow`/`deny` as decimal strings, numbers, or
/// `null` (meaning no permissions). All three map to a bitset here.
fn deserialize_nullable_permissions<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Permissions, D::Error> {
    Option::<Permissions>::deserialize(deserializer).map(|value| value.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shape of a guild text channel as returned by the API.
    const TEXT_CHANNEL: &str = r#"{
        "id": "80351110224678912",
        "type": 0,
        "guild_id": "290926798999268355",
        "position": 6,
        "permission_overwrites": [
            {
                "id": "41771983423143936",
                "type": 1,
                "allow": "0",
                "deny": "1024"
            },
            {
                "id": "338385199178887169",
                "type": 0,
                "allow": "8",
                "deny": null
            }
        ],
        "name": "general",
        "topic": "nothing to see here",
        "nsfw": false,
        "last_message_id": "80351110224678913",
        "rate_limit_per_user": 2,
        "parent_id": "350147556604334080"
    }"#;

    #[test]
    fn text_channel_deserializes() {
        let channel: Channel = serde_json::from_str(TEXT_CHANNEL).unwrap();
        assert_eq!(channel.id.get(), 80_351_110_224_678_912);
        assert_eq!(channel.r#type, ChannelType::GuildText);
        assert_eq!(
            channel.guild_id.map(|g| g.get()),
            Some(290_926_798_999_268_355)
        );
        assert_eq!(channel.position, Some(6));
        assert_eq!(channel.name.as_deref(), Some("general"));
        assert_eq!(channel.rate_limit_per_user, Some(2));
        assert_eq!(
            channel.parent_id.map(|c| c.get()),
            Some(350_147_556_604_334_080)
        );

        let overwrites = channel.permission_overwrites.unwrap();
        assert_eq!(overwrites.len(), 2);
        // Member overwrite (type 1) with deny only.
        assert_eq!(overwrites[0].r#type, OverwriteType::Member);
        assert_eq!(overwrites[0].allow, Permissions::empty());
        assert_eq!(overwrites[0].deny, Permissions::VIEW_CHANNEL);
        // Role overwrite (type 0) with allow; `deny: null` becomes empty.
        assert_eq!(overwrites[1].r#type, OverwriteType::Role);
        assert_eq!(overwrites[1].allow, Permissions::ADMINISTRATOR);
        assert_eq!(overwrites[1].deny, Permissions::empty());
    }

    #[test]
    fn voice_channel_deserializes() {
        let channel: Channel = serde_json::from_str(
            r#"{
                "id": "155101607143833600",
                "type": 2,
                "guild_id": "81384788765712384",
                "position": 1,
                "permission_overwrites": [],
                "name": "General Voice",
                "bitrate": 64000,
                "user_limit": 0,
                "rtc_region": "us-east",
                "video_quality_mode": 2
            }"#,
        )
        .unwrap();
        assert_eq!(channel.r#type, ChannelType::GuildVoice);
        assert_eq!(channel.bitrate, Some(64_000));
        assert_eq!(channel.video_quality_mode, Some(VideoQualityMode::Full));
    }

    #[test]
    fn dm_channel_deserializes() {
        let channel: Channel = serde_json::from_str(
            r#"{
                "id": "351319849649422336",
                "type": 1,
                "last_pin_timestamp": "2016-06-16T23:27:18.849000+00:00",
                "recipients": [
                    { "id": "703411864526716096", "username": "fred", "discriminator": "0" }
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(channel.r#type, ChannelType::Dm);
        assert_eq!(channel.guild_id, None);
        assert_eq!(channel.recipients.map(|r| r.len()), Some(1));
        assert!(channel.last_pin_timestamp.is_some());
    }

    #[test]
    fn unknown_channel_type_is_an_error() {
        // 99 is not a documented channel type.
        assert!(serde_json::from_str::<Channel>(r#"{ "id": "1", "type": 99 }"#).is_err());
    }

    #[test]
    fn channel_roundtrips() {
        let channel: Channel = serde_json::from_str(TEXT_CHANNEL).unwrap();
        let json = serde_json::to_string(&channel).unwrap();
        let reparsed: Channel = serde_json::from_str(&json).unwrap();
        assert_eq!(reparsed, channel);
    }

    #[test]
    fn overwrite_deserializes_from_number_type() {
        let overwrite: PermissionOverwrite =
            serde_json::from_str(r#"{ "id": "41771983423143936", "type": 0, "allow": "2048" }"#)
                .unwrap();
        // `deny` absent -> default empty.
        assert_eq!(overwrite.deny, Permissions::empty());
        assert_eq!(overwrite.r#type, OverwriteType::Role);
        // The ID is generic: same raw value, marker only.
        assert_eq!(overwrite.id.get(), 41_771_983_423_143_936);
    }
}
