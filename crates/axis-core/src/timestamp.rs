//! RFC 3339 timestamps as exchanged with the Discord API.

use core::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/// A point in time, wrapping [`time::OffsetDateTime`].
///
/// Discord exchanges timestamps as RFC 3339 strings such as
/// `2015-04-26T06:26:56.936000+00:00`. Serialization uses that same format,
/// so a `Timestamp` round-trips through JSON unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(OffsetDateTime);

impl Timestamp {
    /// The Unix epoch: 1970-01-01 00:00:00 UTC.
    pub const UNIX_EPOCH: Self = Self(OffsetDateTime::UNIX_EPOCH);

    /// The current time in UTC.
    ///
    /// The system clock is trusted; no attempt is made to compensate for it
    /// running fast or slow.
    pub fn now_utc() -> Self {
        Self(OffsetDateTime::now_utc())
    }

    /// Wrap an existing `OffsetDateTime`.
    pub const fn from_datetime(datetime: OffsetDateTime) -> Self {
        Self(datetime)
    }

    /// Build from a Unix timestamp in milliseconds.
    ///
    /// Returns an error if the value is outside the representable range.
    pub fn from_unix_millis(millis: i64) -> Result<Self, time::error::ComponentRange> {
        OffsetDateTime::from_unix_timestamp_nanos(i128::from(millis) * 1_000_000).map(Self)
    }

    /// The wrapped `OffsetDateTime`.
    pub const fn as_datetime(self) -> OffsetDateTime {
        self.0
    }

    /// The Unix timestamp in milliseconds.
    pub fn as_unix_millis(self) -> i64 {
        (self.0.unix_timestamp_nanos() / 1_000_000) as i64
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0.format(&Rfc3339) {
            Ok(formatted) => f.write_str(&formatted),
            // Formatting can only fail for out-of-range values, which cannot
            // be constructed through the public API; fall back to the raw
            // Unix nanoseconds rather than panicking inside `Display`.
            Err(_) => write!(f, "{}ns", self.0.unix_timestamp_nanos()),
        }
    }
}

impl FromStr for Timestamp {
    type Err = TimestampParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        OffsetDateTime::parse(s, &Rfc3339)
            .map(Self)
            .map_err(|source| TimestampParseError { source })
    }
}

impl Serialize for Timestamp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let formatted = self.0.format(&Rfc3339).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&formatted)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TimestampVisitor;

        impl de::Visitor<'_> for TimestampVisitor {
            type Value = Timestamp;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an RFC 3339 timestamp string")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                value
                    .parse::<Timestamp>()
                    .map_err(|_| E::invalid_value(de::Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_str(TimestampVisitor)
    }
}

/// Error returned when parsing an RFC 3339 timestamp fails.
#[derive(Debug, thiserror::Error)]
#[error("invalid RFC 3339 timestamp")]
pub struct TimestampParseError {
    #[from]
    source: time::error::Parse,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Timestamp format used throughout the Discord API.
    const DISCORD_FORMAT_EXAMPLE: &str = "2015-04-26T06:26:56.936000+00:00";

    #[test]
    fn parses_discord_format() {
        let ts: Timestamp = DISCORD_FORMAT_EXAMPLE.parse().unwrap();
        let dt = ts.as_datetime();
        assert_eq!(dt.year(), 2015);
        assert_eq!(dt.month(), time::Month::April);
        assert_eq!(dt.day(), 26);
        assert_eq!(dt.hour(), 6);
        assert_eq!(dt.minute(), 26);
        assert_eq!(dt.second(), 56);
        assert_eq!(dt.millisecond(), 936);
    }

    #[test]
    fn rejects_garbage() {
        assert!("not a timestamp".parse::<Timestamp>().is_err());
    }

    #[test]
    fn unix_millis_roundtrip() {
        let ts = Timestamp::from_unix_millis(1_462_015_105_796).unwrap();
        assert_eq!(ts.as_unix_millis(), 1_462_015_105_796);
    }

    #[test]
    fn from_unix_millis_rejects_out_of_range() {
        assert!(Timestamp::from_unix_millis(i64::MAX).is_err());
    }

    #[test]
    fn epoch_is_zero() {
        assert_eq!(Timestamp::UNIX_EPOCH.as_unix_millis(), 0);
    }

    #[test]
    fn ordering() {
        let earlier = Timestamp::from_unix_millis(0).unwrap();
        let later = Timestamp::from_unix_millis(1).unwrap();
        assert!(earlier < later);
    }

    #[test]
    fn serde_roundtrip() {
        let ts: Timestamp = DISCORD_FORMAT_EXAMPLE.parse().unwrap();
        let json = serde_json::to_string(&ts).unwrap();
        // Wrapped in quotes: serialized as a string.
        assert!(json.starts_with('"') && json.ends_with('"'));
        let parsed: Timestamp = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ts);
    }

    #[test]
    fn serde_rejects_non_string() {
        assert!(serde_json::from_str::<Timestamp>("123").is_err());
    }
}
