//! The gateway READY payload.

use serde::{Deserialize, Serialize};

use crate::{application::PartialApplication, guild::UnavailableGuild, user::User};

/// The `READY` dispatch: sent once per session after a successful
/// Identify, carrying the bot's user, the unavailable guilds, and the
/// session identity used for resuming.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ready {
    /// The gateway API version in use.
    pub v: Option<u8>,
    /// The bot's user (includes `email` and other authenticated-user
    /// fields).
    pub user: User,
    /// Unavailable guilds at connect time; each becomes available via a
    /// `GUILD_CREATE` dispatch.
    pub guilds: Vec<UnavailableGuild>,
    /// The session ID — required for resuming.
    pub session_id: String,
    /// The gateway URL to use for resume attempts.
    pub resume_gateway_url: Option<String>,
    /// The partial application object.
    pub application: Option<PartialApplication>,
    /// The shard pair sent in Identify, when sharding is used.
    pub shard: Option<[u64; 2]>,
    /// A trace of the gateway servers involved (legacy debugging field).
    pub _trace: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const READY: &str = r#"{
        "v": 10,
        "user": {
            "id": "80351110224678912",
            "username": "axis-test-bot",
            "discriminator": "0",
            "bot": true,
            "verified": true,
            "mfa_enabled": true
        },
        "guilds": [
            { "id": "290926798999268355", "unavailable": true }
        ],
        "session_id": "f0e0c1d2b3a49786",
        "resume_gateway_url": "wss://gateway.discord.gg",
        "application": { "id": "80351110224678912", "flags": 8192 },
        "shard": [0, 2],
        "_trace": ["gateway-bay"]
    }"#;

    #[test]
    fn ready_deserializes() {
        let ready: Ready = serde_json::from_str(READY).unwrap();
        assert_eq!(ready.v, Some(10));
        assert_eq!(ready.user.username, "axis-test-bot");
        assert_eq!(ready.user.bot, Some(true));
        assert_eq!(ready.guilds.len(), 1);
        assert_eq!(ready.session_id, "f0e0c1d2b3a49786");
        assert_eq!(
            ready.resume_gateway_url.as_deref(),
            Some("wss://gateway.discord.gg")
        );
        assert_eq!(
            ready.application.as_ref().map(|app| app.id.get()),
            Some(80_351_110_224_678_912)
        );
        assert_eq!(ready.shard, Some([0, 2]));
    }

    #[test]
    fn ready_roundtrips() {
        let ready: Ready = serde_json::from_str(READY).unwrap();
        let json = serde_json::to_string(&ready).unwrap();
        let reparsed: Ready = serde_json::from_str(&json).unwrap();
        assert_eq!(reparsed, ready);
    }
}
