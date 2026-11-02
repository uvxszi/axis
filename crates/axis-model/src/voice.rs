//! Voice states, typing indicators, and member chunk payloads.

use serde::{Deserialize, Serialize};

use axis_core::{ChannelId, GuildId, Timestamp, UserId};

use crate::member::Member;

/// Where to connect for voice (`VOICE_SERVER_UPDATE`), sent after the
/// bot asks to join a voice channel. Combined with the bot's own
/// [`VoiceState::session_id`] it is everything a voice connection needs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceServerUpdate {
    /// The voice connection token.
    pub token: String,
    /// The guild this voice server is for.
    pub guild_id: GuildId,
    /// The voice server host; `None` while Discord reallocates one.
    pub endpoint: Option<String>,
}

/// A user's voice connection status (`VOICE_STATE_UPDATE`, and the
/// `voice_states` array of `GUILD_CREATE`).
///
/// `channel_id` is `None` when the user left voice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceState {
    /// The guild, absent inside `GUILD_CREATE` (where it is implied).
    pub guild_id: Option<GuildId>,
    /// The voice channel the user is connected to, if any.
    pub channel_id: Option<ChannelId>,
    /// The user this voice state is for.
    pub user_id: UserId,
    /// The guild member, when Discord includes it.
    pub member: Option<Member>,
    /// The voice session ID.
    #[serde(default)]
    pub session_id: String,
    /// Server-deafened.
    #[serde(default)]
    pub deaf: bool,
    /// Server-muted.
    #[serde(default)]
    pub mute: bool,
    /// Locally deafened.
    #[serde(default)]
    pub self_deaf: bool,
    /// Locally muted.
    #[serde(default)]
    pub self_mute: bool,
    /// Streaming ("Go Live").
    pub self_stream: Option<bool>,
    /// Camera on.
    #[serde(default)]
    pub self_video: bool,
    /// Suppressed (stage audience).
    #[serde(default)]
    pub suppress: bool,
    /// When the user requested to speak on a stage.
    pub request_to_speak_timestamp: Option<Timestamp>,
}

/// A user started typing (`TYPING_START`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypingStart {
    /// The channel the user is typing in.
    pub channel_id: ChannelId,
    /// The guild, when applicable.
    pub guild_id: Option<GuildId>,
    /// The typing user.
    pub user_id: UserId,
    /// Unix time (seconds) when typing started.
    pub timestamp: u64,
    /// The member, for guild channels.
    pub member: Option<Member>,
}

/// A chunk of guild members (`GUILD_MEMBERS_CHUNK`), sent in response to
/// a Request Guild Members gateway command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuildMembersChunk {
    /// The guild the members belong to.
    pub guild_id: GuildId,
    /// The members in this chunk.
    pub members: Vec<Member>,
    /// Zero-based index of this chunk.
    pub chunk_index: u32,
    /// Total number of chunks for the request.
    pub chunk_count: u32,
    /// The nonce from the request, if one was set.
    pub nonce: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_state_deserializes() {
        let state: VoiceState = serde_json::from_str(
            r#"{ "channel_id": "157733188964188161", "user_id": "80351110224678912",
                 "session_id": "90326bd25d71d39b9ef95b299e3872ff", "deaf": false, "mute": false,
                 "self_deaf": false, "self_mute": true, "suppress": false,
                 "request_to_speak_timestamp": null, "self_video": false }"#,
        )
        .unwrap();
        assert!(state.self_mute);
        assert_eq!(
            state.channel_id.map(|c| c.get()),
            Some(157_733_188_964_188_161)
        );
        assert_eq!(state.guild_id, None);
    }

    #[test]
    fn left_voice_has_no_channel() {
        let state: VoiceState =
            serde_json::from_str(r#"{ "guild_id": "1", "channel_id": null, "user_id": "2" }"#)
                .unwrap();
        assert_eq!(state.channel_id, None);
    }

    #[test]
    fn typing_start_deserializes() {
        let typing: TypingStart = serde_json::from_str(
            r#"{ "channel_id": "1", "guild_id": "2", "user_id": "3", "timestamp": 1700000000 }"#,
        )
        .unwrap();
        assert_eq!(typing.timestamp, 1_700_000_000);
    }
}
