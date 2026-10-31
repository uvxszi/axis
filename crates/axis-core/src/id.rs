//! Typed IDs for Discord snowflakes.
//!
//! Discord identifies almost every entity with a snowflake: a 64-bit
//! unsigned integer whose upper bits encode the creation timestamp.
//!
//! Axis represents snowflakes with a single generic [`Id<T>`] struct plus
//! zero-sized marker types ([`marker::UserMarker`], [`marker::GuildMarker`],
//! ...). This gives compile-time protection against mixing up a user ID with
//! a channel ID while keeping the implementation to one serde/Display/Hash
//! code path. Short aliases such as [`UserId`] and [`GuildId`] are provided
//! for readability.

use core::{
    fmt::{self, Display, Formatter},
    marker::PhantomData,
    str::FromStr,
};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, Unexpected, Visitor},
};

use crate::timestamp::Timestamp;

/// The Unix timestamp (in milliseconds) of Discord's epoch:
/// 2015-01-01 00:00:00 UTC.
const DISCORD_EPOCH_MILLIS: i64 = 1_420_070_400_000;

/// Number of bits the snowflake's timestamp portion is shifted left by.
const TIMESTAMP_SHIFT: u64 = 22;

/// Zero-sized marker types used to make [`Id`] values entity-specific.
///
/// The markers are named with a `Marker` suffix so that they do not collide
/// with the data models of the same name (for example, the `User` model).
pub mod marker {
    /// Marker for application IDs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ApplicationMarker;

    /// Marker for auto-moderation rule IDs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct AutoModRuleMarker;

    /// Marker for channel IDs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ChannelMarker;

    /// Marker for emoji IDs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct EmojiMarker;

    /// Marker for guild IDs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct GuildMarker;

    /// Marker for integration IDs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct IntegrationMarker;

    /// Marker for interaction IDs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct InteractionMarker;

    /// Marker for message IDs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MessageMarker;

    /// Marker for role IDs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RoleMarker;

    /// Marker for scheduled event IDs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ScheduledEventMarker;

    /// Marker for sticker IDs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct StickerMarker;

    /// Marker for user IDs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct UserMarker;

    /// Marker for IDs that may be either a role ID or a user ID, depending
    /// on context (for example, permission overwrite targets).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct GenericMarker;

    /// Marker for webhook IDs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct WebhookMarker;
}

/// A typed Discord snowflake ID.
///
/// The type parameter `T` is a zero-sized marker (see [`marker`]) that
/// prevents IDs of different entity kinds from being confused:
///
/// ```
/// use axis_core::{ChannelId, UserId};
///
/// let user_id = UserId::new(803_511_102_246_789_12);
/// // let _oops: ChannelId = user_id; // does not compile
/// assert_eq!(user_id.get(), 803_511_102_246_789_12);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id<T> {
    value: u64,
    marker: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// Create a new ID from a raw snowflake value.
    pub const fn new(value: u64) -> Self {
        Self {
            value,
            marker: PhantomData,
        }
    }

    /// The raw snowflake value.
    pub const fn get(self) -> u64 {
        self.value
    }

    /// The creation timestamp encoded in the snowflake.
    ///
    /// Discord snowflakes store the number of milliseconds since
    /// 2015-01-01 00:00:00 UTC in their upper 42 bits; this method decodes
    /// that portion and returns the corresponding [`Timestamp`].
    ///
    /// # Panics
    ///
    /// Never in practice: the decoded value is mathematically guaranteed to
    /// be in range (the largest possible snowflake decodes to a timestamp in
    /// the year 2154). A panic here would indicate a broken build.
    pub fn created_at(self) -> Timestamp {
        let millis = ((self.value >> TIMESTAMP_SHIFT) as i64) + DISCORD_EPOCH_MILLIS;
        Timestamp::from_unix_millis(millis).expect("snowflake timestamp portion is always in range")
    }
}

impl<T> From<u64> for Id<T> {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl<T> FromStr for Id<T> {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s.parse()?))
    }
}

impl<T> Display for Id<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl<T> Serialize for Id<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.value)
    }
}

impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct IdVisitor<T>(PhantomData<fn() -> T>);

        impl<'de, T> Visitor<'de> for IdVisitor<T> {
            type Value = Id<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .write_str("a Discord snowflake (unsigned 64-bit integer or decimal string)")
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(Id::new(value))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                u64::try_from(value).map(Id::new).map_err(|_| {
                    E::invalid_value(Unexpected::Signed(value), &"a non-negative integer")
                })
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                value
                    .parse::<u64>()
                    .map(Id::new)
                    .map_err(|_| E::invalid_value(Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_any(IdVisitor(PhantomData))
    }
}

/// Error returned when parsing a snowflake from a string fails.
#[derive(Debug, thiserror::Error)]
#[error("invalid snowflake ID")]
pub struct ParseIdError(#[from] core::num::ParseIntError);

/// An ID that is either a role ID or a user ID, depending on context.
pub type GenericId = Id<marker::GenericMarker>;
/// An ID of a user.
pub type UserId = Id<marker::UserMarker>;
/// An ID of a guild.
pub type GuildId = Id<marker::GuildMarker>;
/// An ID of a channel.
pub type ChannelId = Id<marker::ChannelMarker>;
/// An ID of a message.
pub type MessageId = Id<marker::MessageMarker>;
/// An ID of a role.
pub type RoleId = Id<marker::RoleMarker>;
/// An ID of an emoji.
pub type EmojiId = Id<marker::EmojiMarker>;
/// An ID of an application.
pub type ApplicationId = Id<marker::ApplicationMarker>;
/// An ID of an interaction.
pub type InteractionId = Id<marker::InteractionMarker>;
/// An ID of an integration.
pub type IntegrationId = Id<marker::IntegrationMarker>;
/// An ID of a webhook.
pub type WebhookId = Id<marker::WebhookMarker>;
/// An ID of a sticker.
pub type StickerId = Id<marker::StickerMarker>;
/// An ID of a scheduled event.
pub type ScheduledEventId = Id<marker::ScheduledEventMarker>;
/// An ID of an auto-moderation rule.
pub type AutoModRuleId = Id<marker::AutoModRuleMarker>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Example snowflake from Discord's documentation, whose timestamp
    /// portion decodes to 2016-04-30 11:18:25.796 UTC.
    const DOC_EXAMPLE: u64 = 175_928_847_299_117_063;

    #[test]
    fn new_and_get() {
        let id = UserId::new(123);
        assert_eq!(id.get(), 123);
        assert_eq!(id, UserId::from(123_u64));
    }

    #[test]
    fn ordering_and_equality_use_raw_value() {
        let a = UserId::new(1);
        let b = GuildId::new(1);
        // Same raw value, different marker types: still comparable by value.
        assert_eq!(a.get(), b.get());
        assert!(UserId::new(2) > UserId::new(1));
    }

    #[test]
    fn created_at_decodes_documented_example() {
        let ts = Id::<marker::UserMarker>::new(DOC_EXAMPLE).created_at();
        let dt = ts.as_datetime();
        assert_eq!(dt.year(), 2016);
        assert_eq!(dt.month(), time::Month::April);
        assert_eq!(dt.day(), 30);
        assert_eq!(dt.hour(), 11);
        assert_eq!(dt.minute(), 18);
        assert_eq!(dt.second(), 25);
        assert_eq!(dt.millisecond(), 796);
    }

    #[test]
    fn created_at_handles_extreme_value() {
        // Largest possible snowflake must not panic (documents the
        // `created_at` invariant).
        let ts = UserId::new(u64::MAX).created_at();
        assert!(ts.as_unix_millis() > DISCORD_EPOCH_MILLIS);
    }

    #[test]
    fn serde_from_number() {
        let id: UserId = serde_json::from_str("80351110224678912").unwrap();
        assert_eq!(id.get(), 80_351_110_224_678_912);
    }

    #[test]
    fn serde_from_string() {
        let id: UserId = serde_json::from_str("\"80351110224678912\"").unwrap();
        assert_eq!(id.get(), 80_351_110_224_678_912);
    }

    #[test]
    fn serde_serializes_as_number() {
        let json = serde_json::to_string(&UserId::new(123)).unwrap();
        assert_eq!(json, "123");
    }

    #[test]
    fn serde_rejects_negative() {
        assert!(serde_json::from_str::<UserId>("-1").is_err());
    }

    #[test]
    fn serde_rejects_garbage_string() {
        assert!(serde_json::from_str::<UserId>("\"not an id\"").is_err());
    }

    #[test]
    fn from_str_roundtrip() {
        let id: UserId = "12345".parse().unwrap();
        assert_eq!(id.to_string(), "12345");
        assert!("abc".parse::<UserId>().is_err());
    }
}
