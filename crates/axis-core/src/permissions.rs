//! Permission bit flags.
//!
//! Permissions are a 64-bit bitset used in roles, channel permission
//! overwrites, and member permission fields. The Discord API serializes
//! permission bitsets as **decimal strings** (for example `"8"` for
//! `ADMINISTRATOR`), so this type serializes to a string and deserializes
//! from either a string or a number for robustness.
//!
//! Source of truth: the official "Permissions" bit table in Discord's
//! documentation.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

bitflags::bitflags! {
    /// A bitset of Discord permission flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Permissions: u64 {
        /// Allows creating guild instant invites.
        const CREATE_INSTANT_INVITE = 1 << 0;
        /// Allows kicking members.
        const KICK_MEMBERS = 1 << 1;
        /// Allows banning members.
        const BAN_MEMBERS = 1 << 2;
        /// Allows all permissions and bypasses channel permission overwrites.
        const ADMINISTRATOR = 1 << 3;
        /// Allows creating, editing, and deleting channels.
        const MANAGE_CHANNELS = 1 << 4;
        /// Allows editing and deleting the guild itself.
        const MANAGE_GUILD = 1 << 5;
        /// Allows adding reactions to messages.
        const ADD_REACTIONS = 1 << 6;
        /// Allows viewing the guild's audit log.
        const VIEW_AUDIT_LOG = 1 << 7;
        /// Allows using priority speaker in voice channels.
        const PRIORITY_SPEAKER = 1 << 8;
        /// Allows streaming video in voice channels.
        const STREAM = 1 << 9;
        /// Allows viewing channels (excluding hidden ones).
        const VIEW_CHANNEL = 1 << 10;
        /// Allows sending messages in channels and threads.
        const SEND_MESSAGES = 1 << 11;
        /// Allows sending text-to-speech messages.
        const SEND_TTS_MESSAGES = 1 << 12;
        /// Allows deleting and pinning other users' messages.
        const MANAGE_MESSAGES = 1 << 13;
        /// Allows links sent by users to show as embeds.
        const EMBED_LINKS = 1 << 14;
        /// Allows uploading files and attachments.
        const ATTACH_FILES = 1 << 15;
        /// Allows reading the message history of channels.
        const READ_MESSAGE_HISTORY = 1 << 16;
        /// Allows using `@everyone` and `@here` mentions and pinging all users in a role.
        const MENTION_EVERYONE = 1 << 17;
        /// Allows using emojis and stickers from other guilds.
        const USE_EXTERNAL_EMOJIS = 1 << 18;
        /// Allows viewing the guild's insights.
        const VIEW_GUILD_INSIGHTS = 1 << 19;
        /// Allows connecting to voice channels.
        const CONNECT = 1 << 20;
        /// Allows speaking in voice channels.
        const SPEAK = 1 << 21;
        /// Allows muting members in voice channels.
        const MUTE_MEMBERS = 1 << 22;
        /// Allows deafening members in voice channels.
        const DEAFEN_MEMBERS = 1 << 23;
        /// Allows moving members between voice channels.
        const MOVE_MEMBERS = 1 << 24;
        /// Allows using voice activity detection.
        const USE_VAD = 1 << 25;
        /// Allows changing one's own nickname.
        const CHANGE_NICKNAME = 1 << 26;
        /// Allows changing other members' nicknames.
        const MANAGE_NICKNAMES = 1 << 27;
        /// Allows creating and editing roles. Shown as "Manage Permissions" in the client.
        const MANAGE_ROLES = 1 << 28;
        /// Allows creating, editing, and deleting webhooks.
        const MANAGE_WEBHOOKS = 1 << 29;
        /// Allows creating, editing, and deleting emojis and stickers.
        const MANAGE_GUILD_EXPRESSIONS = 1 << 30;
        /// Allows using application commands, including slash commands.
        const USE_APPLICATION_COMMANDS = 1 << 31;
        /// Allows requesting to speak in stage channels.
        const REQUEST_TO_SPEAK = 1 << 32;
        /// Allows creating, editing, and deleting scheduled events.
        const MANAGE_EVENTS = 1 << 33;
        /// Allows creating, editing, and deleting threads.
        const MANAGE_THREADS = 1 << 34;
        /// Allows creating public threads.
        const CREATE_PUBLIC_THREADS = 1 << 35;
        /// Allows creating private threads.
        const CREATE_PRIVATE_THREADS = 1 << 36;
        /// Allows using stickers from other guilds.
        const USE_EXTERNAL_STICKERS = 1 << 37;
        /// Allows sending messages in threads.
        const SEND_MESSAGES_IN_THREADS = 1 << 38;
        /// Allows using activities (embedded applications) in voice channels.
        const USE_EMBEDDED_ACTIVITIES = 1 << 39;
        /// Allows timing out members (shown as "Timeout Members" in the client).
        const MODERATE_MEMBERS = 1 << 40;
        /// Allows viewing monetization analytics for the guild.
        const VIEW_CREATOR_MONETIZATION_ANALYTICS = 1 << 41;
        /// Allows using the soundboard in voice channels.
        const USE_SOUNDBOARD = 1 << 42;
        /// Allows creating emojis, stickers, and soundboard sounds.
        const CREATE_GUILD_EXPRESSIONS = 1 << 43;
        /// Allows creating scheduled events.
        const CREATE_EVENTS = 1 << 44;
        /// Allows using custom sounds from other guilds.
        const USE_EXTERNAL_SOUNDS = 1 << 45;
        /// Allows sending voice message attachments.
        const SEND_VOICE_MESSAGES = 1 << 46;
        /// Allows setting the status of a voice channel.
        const SET_VOICE_CHANNEL_STATUS = 1 << 48;
        /// Allows sending polls.
        const SEND_POLLS = 1 << 49;
        /// Allows using external application commands from other applications.
        const USE_EXTERNAL_APPS = 1 << 50;
        /// Allows pinning and unpinning one's own messages.
        const PIN_MESSAGES = 1 << 51;
        /// Allows bypassing channel slowmode.
        const BYPASS_SLOWMODE = 1 << 52;
    }
}

impl Permissions {
    /// Whether this set contains [`ADMINISTRATOR`](Self::ADMINISTRATOR),
    /// which implicitly grants every other permission.
    pub const fn is_administrator(self) -> bool {
        self.contains(Self::ADMINISTRATOR)
    }

    /// Human-readable names of every set permission, in flag declaration
    /// order — for example `["ADMINISTRATOR"]` or
    /// `["SEND_MESSAGES", "EMBED_LINKS"]`.
    ///
    /// Used for error messages and logs; not for API communication.
    pub fn names(self) -> impl Iterator<Item = &'static str> {
        self.iter_names().map(|(name, _)| name)
    }
}

impl Default for Permissions {
    fn default() -> Self {
        Self::empty()
    }
}

impl core::fmt::Display for Permissions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut first = true;
        for name in self.names() {
            if !first {
                f.write_str(", ")?;
            }
            f.write_str(name)?;
            first = false;
        }
        if first {
            f.write_str("(none)")?;
        }
        Ok(())
    }
}

impl Serialize for Permissions {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Discord represents permission bitsets as decimal strings.
        serializer.serialize_str(&self.bits().to_string())
    }
}

impl<'de> Deserialize<'de> for Permissions {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PermissionsVisitor;

        impl de::Visitor<'_> for PermissionsVisitor {
            type Value = Permissions;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter
                    .write_str("a permission bitset as a decimal string or an unsigned integer")
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                // Lenient: Discord may add new bits that older library
                // versions do not know about; unknown bits are kept.
                Ok(Permissions::from_bits_retain(value))
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                let bits: u64 = value.parse().map_err(|_| {
                    E::invalid_value(de::Unexpected::Str(value), &"a decimal permission bitset")
                })?;
                Ok(Permissions::from_bits_retain(bits))
            }
        }

        deserializer.deserialize_any(PermissionsVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documented_bit_values_spot_check() {
        assert_eq!(Permissions::CREATE_INSTANT_INVITE.bits(), 1 << 0);
        assert_eq!(Permissions::ADMINISTRATOR.bits(), 1 << 3);
        assert_eq!(Permissions::SEND_MESSAGES.bits(), 1 << 11);
        assert_eq!(Permissions::MANAGE_GUILD_EXPRESSIONS.bits(), 1 << 30);
        assert_eq!(Permissions::USE_APPLICATION_COMMANDS.bits(), 1 << 31);
        assert_eq!(Permissions::MODERATE_MEMBERS.bits(), 1 << 40);
        assert_eq!(Permissions::SET_VOICE_CHANNEL_STATUS.bits(), 1 << 48);
        assert_eq!(Permissions::SEND_POLLS.bits(), 1 << 49);
        assert_eq!(Permissions::USE_EXTERNAL_APPS.bits(), 1 << 50);
        assert_eq!(Permissions::PIN_MESSAGES.bits(), 1 << 51);
        assert_eq!(Permissions::BYPASS_SLOWMODE.bits(), 1 << 52);
        // Bits 47 is not defined by Discord; nothing at 1 << 47.
        assert_eq!(Permissions::from_bits_retain(1 << 47).bits(), 1 << 47);
    }

    #[test]
    fn administrator_check() {
        assert!(Permissions::ADMINISTRATOR.is_administrator());
        assert!(!(Permissions::SEND_MESSAGES | Permissions::MANAGE_GUILD).is_administrator());
        assert!(!Permissions::empty().is_administrator());
    }

    #[test]
    fn names_lists_set_permissions() {
        let perms = Permissions::SEND_MESSAGES | Permissions::EMBED_LINKS;
        let names: Vec<_> = perms.names().collect();
        assert_eq!(names, vec!["SEND_MESSAGES", "EMBED_LINKS"]);
        assert_eq!(Permissions::empty().names().count(), 0);
    }

    #[test]
    fn display_joins_names() {
        let perms = Permissions::ADMINISTRATOR;
        assert_eq!(perms.to_string(), "ADMINISTRATOR");
        assert_eq!(Permissions::empty().to_string(), "(none)");
    }

    #[test]
    fn serde_serializes_as_string() {
        // 8 == ADMINISTRATOR, the form Discord actually uses.
        let json = serde_json::to_string(&Permissions::ADMINISTRATOR).unwrap();
        assert_eq!(json, "\"8\"");
    }

    #[test]
    fn serde_deserializes_from_string() {
        let perms: Permissions = serde_json::from_str("\"2048\"").unwrap();
        assert_eq!(perms, Permissions::SEND_MESSAGES);
    }

    #[test]
    fn serde_deserializes_from_number() {
        let perms: Permissions = serde_json::from_str("8").unwrap();
        assert_eq!(perms, Permissions::ADMINISTRATOR);
    }

    #[test]
    fn serde_rejects_garbage_string() {
        assert!(serde_json::from_str::<Permissions>("\"nope\"").is_err());
    }

    #[test]
    fn serde_keeps_unknown_bits() {
        // A future permission Discord adds should not break deserialization.
        // 2^53 + 1: low bit (CREATE_INSTANT_INVITE) plus an unknown high bit.
        let perms: Permissions = serde_json::from_str("\"9007199254740993\"").unwrap();
        assert!(perms.contains(Permissions::CREATE_INSTANT_INVITE));
        assert_eq!(perms.bits(), 0x0020_0000_0000_0001);
    }
}
