//! Gateway intents.
//!
//! Intents are bit masks an application sends in its `Identify` payload to
//! declare which groups of gateway events it wants to receive. Sending an
//! intent value Discord does not recognize causes the connection to be
//! closed with close code 4013.
//!
//! Source of truth: the official "List of Intents" table in Discord's
//! Gateway documentation.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

bitflags::bitflags! {
    /// A bitset of gateway event subscriptions.
    ///
    /// ```
    /// use axis_core::GatewayIntents;
    ///
    /// let intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES;
    /// assert!(intents.contains(GatewayIntents::GUILDS));
    /// assert!(!intents.is_privileged());
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct GatewayIntents: u64 {
        /// Tracks guild creation/update/deletion, channels, threads, roles, and related events.
        const GUILDS = 1 << 0;
        /// Tracks member join/leave/update events. **Privileged.**
        const GUILD_MEMBERS = 1 << 1;
        /// Tracks bans, unbans, and audit log events.
        const GUILD_MODERATION = 1 << 2;
        /// Tracks emoji and sticker creation/update/deletion. Formerly `GUILD_EMOJIS_AND_STICKERS`.
        const GUILD_EXPRESSIONS = 1 << 3;
        /// Tracks integration creation/update/deletion.
        const GUILD_INTEGRATIONS = 1 << 4;
        /// Tracks webhook creation/update/deletion.
        const GUILD_WEBHOOKS = 1 << 5;
        /// Tracks invite creation/update/deletion.
        const GUILD_INVITES = 1 << 6;
        /// Tracks voice state updates (members joining/leaving/muting in voice channels).
        const GUILD_VOICE_STATES = 1 << 7;
        /// Tracks presence updates. **Privileged.**
        const GUILD_PRESENCES = 1 << 8;
        /// Tracks message send/edit/delete in guilds.
        const GUILD_MESSAGES = 1 << 9;
        /// Tracks reaction add/remove in guilds.
        const GUILD_MESSAGE_REACTIONS = 1 << 10;
        /// Tracks typing indicator events in guilds.
        const GUILD_MESSAGE_TYPING = 1 << 11;
        /// Tracks message send/edit/delete in direct messages.
        const DIRECT_MESSAGES = 1 << 12;
        /// Tracks reaction add/remove in direct messages.
        const DIRECT_MESSAGE_REACTIONS = 1 << 13;
        /// Tracks typing indicator events in direct messages.
        const DIRECT_MESSAGE_TYPING = 1 << 14;
        /// Receives message `content`, `attachments`, `embeds`, and `components` in most messages. **Privileged.**
        const MESSAGE_CONTENT = 1 << 15;
        /// Tracks guild scheduled event creation/update/deletion and user participation.
        const GUILD_SCHEDULED_EVENTS = 1 << 16;
        /// Tracks auto-moderation rule creation/update/deletion.
        const AUTO_MODERATION_CONFIGURATION = 1 << 20;
        /// Tracks auto-moderation rule executions (messages blocked/flagged).
        const AUTO_MODERATION_EXECUTION = 1 << 21;
        /// Tracks poll votes and poll updates in guilds.
        const GUILD_MESSAGE_POLLS = 1 << 24;
        /// Tracks poll votes and poll updates in direct messages.
        const DIRECT_MESSAGE_POLLS = 1 << 25;
    }
}

impl GatewayIntents {
    /// The three privileged intents: [`GUILD_PRESENCES`](Self::GUILD_PRESENCES),
    /// [`GUILD_MEMBERS`](Self::GUILD_MEMBERS), and
    /// [`MESSAGE_CONTENT`](Self::MESSAGE_CONTENT).
    ///
    /// These must be explicitly enabled for the application in the Discord
    /// Developer Portal (and approved by Discord for apps in 100+ guilds);
    /// otherwise the gateway closes the connection with close code 4014.
    pub const fn privileged() -> Self {
        Self::from_bits_retain(
            Self::GUILD_PRESENCES.bits()
                | Self::GUILD_MEMBERS.bits()
                | Self::MESSAGE_CONTENT.bits(),
        )
    }

    /// All intents that do **not** require privileged approval.
    pub const fn unprivileged() -> Self {
        Self::from_bits_retain(Self::all().bits() & !Self::privileged().bits())
    }

    /// Whether this set contains at least one privileged intent.
    pub const fn is_privileged(self) -> bool {
        self.intersects(Self::privileged())
    }
}

impl Serialize for GatewayIntents {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // The Identify payload carries intents as a JSON number.
        serializer.serialize_u64(self.bits())
    }
}

impl<'de> Deserialize<'de> for GatewayIntents {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct IntentsVisitor;

        impl de::Visitor<'_> for IntentsVisitor {
            type Value = GatewayIntents;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("a gateway intents bit value (u64)")
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                // Strict: unknown bits would make Discord close the
                // connection with 4013, so reject them here instead.
                GatewayIntents::from_bits(value).ok_or_else(|| {
                    E::invalid_value(
                        de::Unexpected::Unsigned(value),
                        &"a valid combination of gateway intent bits",
                    )
                })
            }
        }

        deserializer.deserialize_u64(IntentsVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documented_bit_values() {
        assert_eq!(GatewayIntents::GUILDS.bits(), 1 << 0);
        assert_eq!(GatewayIntents::GUILD_MEMBERS.bits(), 1 << 1);
        assert_eq!(GatewayIntents::GUILD_MODERATION.bits(), 1 << 2);
        assert_eq!(GatewayIntents::GUILD_EXPRESSIONS.bits(), 1 << 3);
        assert_eq!(GatewayIntents::GUILD_INTEGRATIONS.bits(), 1 << 4);
        assert_eq!(GatewayIntents::GUILD_WEBHOOKS.bits(), 1 << 5);
        assert_eq!(GatewayIntents::GUILD_INVITES.bits(), 1 << 6);
        assert_eq!(GatewayIntents::GUILD_VOICE_STATES.bits(), 1 << 7);
        assert_eq!(GatewayIntents::GUILD_PRESENCES.bits(), 1 << 8);
        assert_eq!(GatewayIntents::GUILD_MESSAGES.bits(), 1 << 9);
        assert_eq!(GatewayIntents::GUILD_MESSAGE_REACTIONS.bits(), 1 << 10);
        assert_eq!(GatewayIntents::GUILD_MESSAGE_TYPING.bits(), 1 << 11);
        assert_eq!(GatewayIntents::DIRECT_MESSAGES.bits(), 1 << 12);
        assert_eq!(GatewayIntents::DIRECT_MESSAGE_REACTIONS.bits(), 1 << 13);
        assert_eq!(GatewayIntents::DIRECT_MESSAGE_TYPING.bits(), 1 << 14);
        assert_eq!(GatewayIntents::MESSAGE_CONTENT.bits(), 1 << 15);
        assert_eq!(GatewayIntents::GUILD_SCHEDULED_EVENTS.bits(), 1 << 16);
        assert_eq!(
            GatewayIntents::AUTO_MODERATION_CONFIGURATION.bits(),
            1 << 20
        );
        assert_eq!(GatewayIntents::AUTO_MODERATION_EXECUTION.bits(), 1 << 21);
        assert_eq!(GatewayIntents::GUILD_MESSAGE_POLLS.bits(), 1 << 24);
        assert_eq!(GatewayIntents::DIRECT_MESSAGE_POLLS.bits(), 1 << 25);
    }

    #[test]
    fn privileged_intents() {
        assert_eq!(
            GatewayIntents::privileged(),
            GatewayIntents::GUILD_PRESENCES
                | GatewayIntents::GUILD_MEMBERS
                | GatewayIntents::MESSAGE_CONTENT
        );
        assert!(GatewayIntents::MESSAGE_CONTENT.is_privileged());
        assert!(GatewayIntents::GUILD_MEMBERS.is_privileged());
        assert!(GatewayIntents::GUILD_PRESENCES.is_privileged());
        assert!(!GatewayIntents::GUILDS.is_privileged());
        assert!(!GatewayIntents::unprivileged().is_privileged());
    }

    #[test]
    fn unprivileged_plus_privileged_is_all() {
        assert_eq!(
            GatewayIntents::unprivileged() | GatewayIntents::privileged(),
            GatewayIntents::all()
        );
    }

    #[test]
    fn serde_roundtrip() {
        let intents = GatewayIntents::GUILDS | GatewayIntents::MESSAGE_CONTENT;
        let json = serde_json::to_string(&intents).unwrap();
        assert_eq!(json, "32769");
        let parsed: GatewayIntents = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, intents);
    }

    #[test]
    fn serde_rejects_unknown_bits() {
        // Bit 17-19, 22-23 etc. are not defined intents.
        assert!(serde_json::from_str::<GatewayIntents>("131072").is_err());
        assert!(serde_json::from_str::<GatewayIntents>("0").is_ok());
    }
}
