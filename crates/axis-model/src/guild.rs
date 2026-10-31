//! The Discord guild (server) object.

use serde::{Deserialize, Serialize};

use axis_core::{ApplicationId, ChannelId, GuildId, Timestamp, UserId};

use crate::{channel::Channel, emoji::Emoji, member::Member, role::Role};

api_enum! {
    /// Verification level required for a guild's members.
    VerificationLevel: u8 {
        /// No verification.
        None = 0,
        /// Must have a verified email.
        Low = 1,
        /// Must be registered on Discord for longer than 5 minutes.
        Medium = 2,
        /// Must be a member for longer than 10 minutes.
        High = 3,
        /// Must have a verified phone number.
        VeryHigh = 4,
    }
}

api_enum! {
    /// Default message notification level of a guild.
    DefaultMessageNotificationLevel: u8 {
        /// Notify for all messages.
        AllMessages = 0,
        /// Notify only for mentions.
        OnlyMentions = 1,
    }
}

api_enum! {
    /// Explicit content filter level of a guild.
    ExplicitContentFilter: u8 {
        /// Don't scan any messages.
        Disabled = 0,
        /// Scan messages from members without a role.
        MembersWithoutRoles = 1,
        /// Scan messages from all members.
        AllMembers = 2,
    }
}

api_enum! {
    /// Whether moderation actions require two-factor authentication.
    MfaLevel: u8 {
        /// 2FA not required.
        None = 0,
        /// 2FA required for moderation actions.
        Elevated = 1,
    }
}

api_enum! {
    /// Age-restriction level of a guild.
    NsfwLevel: u8 {
        /// Not classified.
        Default = 0,
        /// Explicit content.
        Explicit = 1,
        /// Safe content.
        Safe = 2,
        /// Age-restricted content.
        AgeRestricted = 3,
    }
}

api_enum! {
    /// The boost level of a guild.
    PremiumTier: u8 {
        /// No boost level.
        None = 0,
        /// Boost level 1.
        Tier1 = 1,
        /// Boost level 2.
        Tier2 = 2,
        /// Boost level 3.
        Tier3 = 3,
    }
}

/// A guild (server), in the shape of the standard guild object.
///
/// Fields that only appear in enriched variants (`with_counts`) or other
/// endpoints are optional here. Guild "features" are free-form strings
/// that Discord extends over time and are kept as-is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Guild {
    /// The guild ID.
    pub id: GuildId,
    /// The guild name (2–100 characters).
    pub name: String,
    /// The guild icon image hash, if set.
    pub icon: Option<String>,
    /// The guild splash image hash, if set.
    pub splash: Option<String>,
    /// The discovery splash image hash, if set.
    pub discovery_splash: Option<String>,
    /// The ID of the guild owner.
    pub owner_id: UserId,
    /// The ID of the AFK voice channel, if set.
    pub afk_channel_id: Option<ChannelId>,
    /// Seconds a member must be idle before being moved to the AFK channel.
    pub afk_timeout: u32,
    /// The verification level required for members.
    pub verification_level: VerificationLevel,
    /// The default message notification level.
    pub default_message_notifications: DefaultMessageNotificationLevel,
    /// The explicit content filter level.
    pub explicit_content_filter: ExplicitContentFilter,
    /// The guild's roles, including the `@everyone` role.
    pub roles: Vec<Role>,
    /// The guild's custom emojis.
    pub emojis: Vec<Emoji>,
    /// Discord guild features, as free-form strings.
    pub features: Vec<String>,
    /// Whether moderation actions require two-factor authentication.
    pub mfa_level: MfaLevel,
    /// The application that created this guild, if any.
    pub application_id: Option<ApplicationId>,
    /// The channel where system messages (welcome, boost) are posted.
    pub system_channel_id: Option<ChannelId>,
    /// The rules channel for community guilds.
    pub rules_channel_id: Option<ChannelId>,
    /// The vanity invite URL code, if set.
    pub vanity_url_code: Option<String>,
    /// The guild description, if set.
    pub description: Option<String>,
    /// The guild banner image hash, if set.
    pub banner: Option<String>,
    /// The boost tier.
    pub premium_tier: PremiumTier,
    /// The number of boosters, when Discord includes it.
    pub premium_subscription_count: Option<u32>,
    /// The preferred locale of community guilds.
    pub preferred_locale: String,
    /// The channel for community updates, if set.
    pub public_updates_channel_id: Option<ChannelId>,
    /// The age-restriction level.
    pub nsfw_level: NsfwLevel,
    /// Whether the boost progress bar is enabled.
    pub premium_progress_bar_enabled: bool,
    /// The safety alerts channel for community guilds.
    pub safety_alerts_channel_id: Option<ChannelId>,
}

/// A guild that is (temporarily) unavailable, as sent in READY and
/// `GUILD_DELETE`.
///
/// `unavailable` is `Some(true)` for outages; in `GUILD_DELETE` the field
/// is absent when the bot left or was removed from the guild.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnavailableGuild {
    /// The guild ID.
    pub id: GuildId,
    /// Whether the guild is merely unavailable (outage) rather than gone.
    pub unavailable: Option<bool>,
}

/// The `GUILD_CREATE` payload: a standard guild object with connection
/// extras flattened in.
///
/// Depending on the bot's intents and the guild's size, `members` and
/// `presences` may contain only a subset of the guild (the docs spell out
/// the limits); other embedded collections (`voice_states`, `presences`,
/// `stage_instances`, `guild_scheduled_events`, `soundboard_sounds`) are
/// not modeled yet and are ignored on deserialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuildCreate {
    /// The guild fields, flattened into the same payload object.
    #[serde(flatten)]
    pub guild: Guild,
    /// When the bot's user joined the guild.
    pub joined_at: Option<Timestamp>,
    /// Whether the guild is considered "large".
    pub large: Option<bool>,
    /// Whether the guild is currently unavailable (outage recovery).
    pub unavailable: Option<bool>,
    /// The member count at dispatch time.
    pub member_count: Option<u64>,
    /// The guild's members, when Discord included them.
    pub members: Option<Vec<Member>>,
    /// The guild's channels, when Discord included them.
    pub channels: Option<Vec<Channel>>,
    /// Active threads the bot can view, when Discord included them.
    pub threads: Option<Vec<Channel>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed guild payload in the standard shape.
    const GUILD: &str = r#"{
        "id": "290926798999268355",
        "name": "Mason's Server",
        "icon": "3cf945aec3e1c1cde9e973f2dd7aecc9",
        "splash": null,
        "owner_id": "87616711559942144",
        "afk_channel_id": null,
        "afk_timeout": 300,
        "verification_level": 2,
        "default_message_notifications": 1,
        "explicit_content_filter": 0,
        "roles": [
            {
                "id": "290926798999268355",
                "name": "@everyone",
                "color": 0,
                "colors": { "primary_color": 0 },
                "hoist": false,
                "position": 0,
                "permissions": "104324673",
                "managed": false,
                "mentionable": false
            }
        ],
        "emojis": [
            { "id": "417719834621489152", "name": "ultra_daruma", "animated": true }
        ],
        "features": ["COMMUNITY", "ROLE_ICONS"],
        "mfa_level": 0,
        "application_id": null,
        "system_channel_id": "417722298349095947",
        "rules_channel_id": null,
        "vanity_url_code": null,
        "description": null,
        "banner": null,
        "premium_tier": 1,
        "premium_subscription_count": 2,
        "preferred_locale": "en-US",
        "public_updates_channel_id": null,
        "nsfw_level": 2,
        "premium_progress_bar_enabled": false,
        "safety_alerts_channel_id": null
    }"#;

    #[test]
    fn guild_deserializes() {
        let guild: Guild = serde_json::from_str(GUILD).unwrap();
        assert_eq!(guild.id.get(), 290_926_798_999_268_355);
        assert_eq!(guild.name, "Mason's Server");
        assert_eq!(guild.owner_id.get(), 87_616_711_559_942_144);
        assert_eq!(guild.verification_level, VerificationLevel::Medium);
        assert_eq!(
            guild.default_message_notifications,
            DefaultMessageNotificationLevel::OnlyMentions
        );
        assert_eq!(guild.afk_timeout, 300);
        assert_eq!(guild.roles.len(), 1);
        assert_eq!(guild.roles[0].name, "@everyone");
        // The @everyone role's ID equals the guild ID.
        assert_eq!(guild.roles[0].id.get(), guild.id.get());
        assert_eq!(guild.emojis.len(), 1);
        assert_eq!(guild.emojis[0].name.as_deref(), Some("ultra_daruma"));
        assert_eq!(guild.features, vec!["COMMUNITY", "ROLE_ICONS"]);
        assert_eq!(guild.premium_tier, PremiumTier::Tier1);
        assert_eq!(guild.nsfw_level, NsfwLevel::Safe);
        assert_eq!(guild.preferred_locale, "en-US");
    }

    #[test]
    fn guild_ignores_unknown_fields() {
        // The with_counts variant adds fields; a stale model must survive.
        let json = GUILD.trim_end_matches('}').to_owned()
            + r#", "approximate_member_count": 5, "approximate_presence_count": 2}"#;
        let guild: Guild = serde_json::from_str(&json).unwrap();
        assert_eq!(guild.name, "Mason's Server");
    }

    #[test]
    fn guild_roundtrips() {
        let guild: Guild = serde_json::from_str(GUILD).unwrap();
        let json = serde_json::to_string(&guild).unwrap();
        let reparsed: Guild = serde_json::from_str(&json).unwrap();
        assert_eq!(reparsed, guild);
    }

    #[test]
    fn guild_create_flattens_guild_fields() {
        // GUILD_CREATE embeds the guild object plus connection extras.
        let create: GuildCreate = serde_json::from_str(
            r#"{
                "id": "290926798999268355",
                "name": "Mason's Server",
                "owner_id": "87616711559942144",
                "afk_timeout": 300,
                "verification_level": 0,
                "default_message_notifications": 0,
                "explicit_content_filter": 0,
                "roles": [],
                "emojis": [],
                "features": [],
                "mfa_level": 0,
                "premium_tier": 0,
                "preferred_locale": "en-US",
                "nsfw_level": 0,
                "premium_progress_bar_enabled": false,
                "joined_at": "2015-04-26T06:26:56.936000+00:00",
                "large": false,
                "member_count": 2,
                "members": [],
                "channels": [
                    { "id": "417722298349095947", "type": 0, "name": "general" }
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(create.guild.name, "Mason's Server");
        assert_eq!(create.member_count, Some(2));
        assert_eq!(create.large, Some(false));
        assert_eq!(create.channels.as_ref().unwrap().len(), 1);
        assert_eq!(
            create.joined_at.as_ref().unwrap().as_datetime().year(),
            2015
        );
    }

    #[test]
    fn unavailable_guild() {
        let g: UnavailableGuild =
            serde_json::from_str(r#"{ "id": "5", "unavailable": true }"#).unwrap();
        assert_eq!(g.unavailable, Some(true));
        // GUILD_DELETE after a kick: `unavailable` absent.
        let g: UnavailableGuild = serde_json::from_str(r#"{ "id": "5" }"#).unwrap();
        assert_eq!(g.unavailable, None);
    }
}
