//! The Discord user object and its flags.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use axis_core::id::UserId;

bitflags::bitflags! {
    /// Public flags describing what a user is or has achieved.
    ///
    /// Source of truth: the "User Flags" table in Discord's User resource
    /// documentation. Deserialization is lenient: unknown bits (Discord may
    /// add flags) are dropped rather than rejected.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct UserFlags: u64 {
        /// Discord Employee.
        const STAFF = 1 << 0;
        /// Partnered Server Owner.
        const PARTNER = 1 << 1;
        /// HypeSquad Events Member.
        const HYPESQUAD = 1 << 2;
        /// Bug Hunter Level 1.
        const BUG_HUNTER_LEVEL_1 = 1 << 3;
        /// House Bravery Member.
        const HYPESQUAD_ONLINE_HOUSE_1 = 1 << 6;
        /// House Brilliance Member.
        const HYPESQUAD_ONLINE_HOUSE_2 = 1 << 7;
        /// House Balance Member.
        const HYPESQUAD_ONLINE_HOUSE_3 = 1 << 8;
        /// Early Nitro Supporter.
        const PREMIUM_EARLY_SUPPORTER = 1 << 9;
        /// The user is a team (team user accounts).
        const TEAM_PSEUDO_USER = 1 << 10;
        /// Bug Hunter Level 2.
        const BUG_HUNTER_LEVEL_2 = 1 << 14;
        /// Verified Bot.
        const VERIFIED_BOT = 1 << 16;
        /// Early Verified Bot Developer.
        const VERIFIED_DEVELOPER = 1 << 17;
        /// Moderator Programs Alumni (Certified Moderator).
        const CERTIFIED_MODERATOR = 1 << 18;
        /// Bot uses only HTTP interactions and is shown in the online member list.
        const BOT_HTTP_INTERACTIONS = 1 << 19;
    }
}

impl Serialize for UserFlags {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.bits())
    }
}

impl<'de> Deserialize<'de> for UserFlags {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FlagsVisitor;

        impl de::Visitor<'_> for FlagsVisitor {
            type Value = UserFlags;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("a user flags bit value (u64)")
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                // Lenient: keep known bits, silently drop unknown ones.
                Ok(UserFlags::from_bits_truncate(value))
            }
        }

        deserializer.deserialize_u64(FlagsVisitor)
    }
}

/// The type of Nitro subscription a user has.
///
/// Source of truth: the "Premium Type" enum in Discord's User resource
/// documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PremiumType {
    /// No subscription.
    None = 0,
    /// Nitro Classic.
    NitroClassic = 1,
    /// Nitro.
    Nitro = 2,
    /// Nitro Basic.
    Basic = 3,
}

impl PremiumType {
    /// Parse a premium type from its numeric API value.
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::NitroClassic),
            2 => Some(Self::Nitro),
            3 => Some(Self::Basic),
            _ => None,
        }
    }
}

impl Serialize for PremiumType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for PremiumType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PremiumTypeVisitor;

        impl de::Visitor<'_> for PremiumTypeVisitor {
            type Value = PremiumType;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("a premium type value (0-3)")
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                // serde_json reports all JSON integers through `visit_u64`,
                // even when a narrower width was requested.
                let value = u8::try_from(value).map_err(|_| {
                    E::invalid_value(
                        de::Unexpected::Unsigned(value),
                        &"a known premium type value",
                    )
                })?;
                PremiumType::from_u8(value).ok_or_else(|| {
                    E::invalid_value(
                        de::Unexpected::Unsigned(u64::from(value)),
                        &"a known premium type value",
                    )
                })
            }
        }

        deserializer.deserialize_u64(PremiumTypeVisitor)
    }
}

/// A Discord user.
///
/// Field presence mirrors the official User Object documentation: fields
/// marked optional or nullable there are `Option` here. The same user
/// object appears in many API contexts with different subsets of fields
/// populated (for example, `Message.author` only ever includes the fields
/// Discord chooses to include in that context), so consumers must treat
/// every `Option` as potentially `None` regardless of where the object came
/// from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct User {
    /// The user's ID.
    pub id: UserId,
    /// The user's username, unique to the user (Discord's migration to
    /// unique usernames means new accounts always have `discriminator` "0").
    pub username: String,
    /// The user's 4-digit legacy discriminator. `"0"` for users on the
    /// unique-username system.
    pub discriminator: String,
    /// The user's display name, if set.
    pub global_name: Option<String>,
    /// The user's avatar image hash, if set.
    pub avatar: Option<String>,
    /// Whether the user is a bot.
    pub bot: Option<bool>,
    /// Whether the user is an official Discord system agent.
    pub system: Option<bool>,
    /// Whether the user has two-factor authentication enabled on their account.
    pub mfa_enabled: Option<bool>,
    /// The user's banner image hash, if set.
    pub banner: Option<String>,
    /// The user's banner accent color, as a `0xRRGGBB` integer, if set.
    pub accent_color: Option<u32>,
    /// The user's chosen language option.
    pub locale: Option<String>,
    /// Whether the email on the account has been verified.
    pub verified: Option<bool>,
    /// The user's email address. Always empty for bots and other users
    /// besides the authenticated one.
    pub email: Option<String>,
    /// The user's flags.
    pub flags: Option<UserFlags>,
    /// The type of Nitro subscription on the user's account.
    pub premium_type: Option<PremiumType>,
    /// The user's public flags.
    pub public_flags: Option<UserFlags>,
}

impl User {
    /// The mention string for this user, e.g. `<@80351110224678912>`.
    pub fn mention(&self) -> String {
        format!("<@{}>", self.id)
    }

    /// Whether this is a bot account.
    ///
    /// `false` when the field is absent (the API omits it for human users
    /// in some contexts).
    pub fn is_bot(&self) -> bool {
        self.bot.unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shape of a full user object as returned by e.g. `GET /users/@me`.
    const FULL_USER: &str = r#"{
        "id": "80351110224678912",
        "username": "Nelly",
        "discriminator": "0",
        "global_name": "Nelly",
        "avatar": "3370dadd186e2fcd8e5d22feb5674c88",
        "bot": false,
        "system": false,
        "mfa_enabled": true,
        "banner": "a_1e1b4beddd7d39c9cd882b5eb7a0e27a",
        "accent_color": 16739022,
        "locale": "en-US",
        "verified": true,
        "email": "nelly@discordapp.com",
        "flags": 64,
        "premium_type": 2,
        "public_flags": 131072
    }"#;

    #[test]
    fn full_user_deserializes() {
        let user: User = serde_json::from_str(FULL_USER).unwrap();

        // The ID arrived as a string; typed as u64 underneath.
        assert_eq!(user.id.get(), 80_351_110_224_678_912);
        assert_eq!(user.username, "Nelly");
        assert_eq!(user.discriminator, "0");
        assert_eq!(user.global_name.as_deref(), Some("Nelly"));
        assert_eq!(user.bot, Some(false));
        assert_eq!(user.mfa_enabled, Some(true));
        assert_eq!(user.accent_color, Some(16_739_022));
        assert_eq!(user.locale.as_deref(), Some("en-US"));
        assert_eq!(user.email.as_deref(), Some("nelly@discordapp.com"));
    }

    #[test]
    fn flags_decode_correctly() {
        let user: User = serde_json::from_str(FULL_USER).unwrap();
        // 64 == 1 << 6 == HYPESQUAD_ONLINE_HOUSE_1.
        assert_eq!(user.flags, Some(UserFlags::HYPESQUAD_ONLINE_HOUSE_1));
        // 131072 == 1 << 17 == VERIFIED_DEVELOPER.
        assert_eq!(user.public_flags, Some(UserFlags::VERIFIED_DEVELOPER));
    }

    #[test]
    fn premium_type_decodes_correctly() {
        let user: User = serde_json::from_str(FULL_USER).unwrap();
        assert_eq!(user.premium_type, Some(PremiumType::Nitro));
    }

    #[test]
    fn partial_user_leaves_options_empty() {
        // Message.author-style minimal payload.
        let user: User = serde_json::from_str(
            r#"{
                "id": 80351110224678912,
                "username": "minimal",
                "discriminator": "0"
            }"#,
        )
        .unwrap();

        assert_eq!(user.username, "minimal");
        assert_eq!(user.global_name, None);
        assert_eq!(user.avatar, None);
        assert_eq!(user.bot, None);
        assert!(!user.is_bot());
        assert_eq!(user.flags, None);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        // Discord adds new fields over time; they must not break parsing.
        let user: User = serde_json::from_str(
            r#"{
                "id": "123",
                "username": "x",
                "discriminator": "0",
                "some_future_field": { "nested": true }
            }"#,
        )
        .unwrap();

        assert_eq!(user.id.get(), 123);
    }

    #[test]
    fn missing_required_field_is_an_error() {
        // `id` missing: this is a real problem and must surface.
        assert!(
            serde_json::from_str::<User>(r#"{ "username": "x", "discriminator": "0" }"#).is_err()
        );
    }

    #[test]
    fn user_roundtrips_through_serde() {
        let user: User = serde_json::from_str(FULL_USER).unwrap();
        let json = serde_json::to_string(&user).unwrap();
        let reparsed: User = serde_json::from_str(&json).unwrap();
        assert_eq!(reparsed, user);
    }

    #[test]
    fn mention_format() {
        let user: User = serde_json::from_str(
            r#"{ "id": "80351110224678912", "username": "N", "discriminator": "0" }"#,
        )
        .unwrap();
        assert_eq!(user.mention(), "<@80351110224678912>");
    }

    #[test]
    fn user_flags_serde() {
        let json = serde_json::to_string(&UserFlags::VERIFIED_BOT).unwrap();
        assert_eq!(json, "65536");
        let parsed: UserFlags = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, UserFlags::VERIFIED_BOT);
    }

    #[test]
    fn user_flags_drop_unknown_bits() {
        // A flag Discord adds in the future must not break old clients.
        let parsed: UserFlags = serde_json::from_str("1048576").unwrap(); // 1 << 20, undocumented
        assert_eq!(parsed.bits(), 0);
        // Known bits survive alongside unknown ones.
        let parsed: UserFlags = serde_json::from_str("65537").unwrap(); // STAFF | 1 << 16
        assert_eq!(parsed, UserFlags::STAFF | UserFlags::VERIFIED_BOT);
    }

    #[test]
    fn premium_type_values() {
        assert_eq!(PremiumType::from_u8(0), Some(PremiumType::None));
        assert_eq!(PremiumType::from_u8(1), Some(PremiumType::NitroClassic));
        assert_eq!(PremiumType::from_u8(2), Some(PremiumType::Nitro));
        assert_eq!(PremiumType::from_u8(3), Some(PremiumType::Basic));
        assert_eq!(PremiumType::from_u8(4), None);

        let json = serde_json::to_string(&PremiumType::Basic).unwrap();
        assert_eq!(json, "3");
        assert!(serde_json::from_str::<PremiumType>("9").is_err());
    }
}
