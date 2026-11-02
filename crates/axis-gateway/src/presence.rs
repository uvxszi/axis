//! Client-sent gateway commands: presence updates, voice state updates,
//! and guild member requests, plus the [`ShardMessenger`] handle that
//! queues them onto a running shard.

use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use axis_core::{
    ChannelId, ConnectionError, GuildId, UserId,
    error::{AxisError, Result},
};

use crate::protocol::{Opcode, Payload};

/// The bot's online status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OnlineStatus {
    /// Online (green).
    Online,
    /// Do Not Disturb (red).
    Dnd,
    /// Idle (yellow).
    Idle,
    /// Shown as offline.
    Invisible,
}

/// The kind of an activity shown under the bot's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivityKind {
    /// "Playing {name}".
    Playing,
    /// "Streaming {name}" (requires a Twitch/YouTube URL).
    Streaming,
    /// "Listening to {name}".
    Listening,
    /// "Watching {name}".
    Watching,
    /// A custom status: the `state` text is shown as-is.
    Custom,
    /// "Competing in {name}".
    Competing,
}

impl ActivityKind {
    const fn value(self) -> u8 {
        match self {
            Self::Playing => 0,
            Self::Streaming => 1,
            Self::Listening => 2,
            Self::Watching => 3,
            Self::Custom => 4,
            Self::Competing => 5,
        }
    }
}

/// An activity shown under the bot's name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Activity {
    /// The activity kind.
    pub kind: ActivityKind,
    /// The activity name (for custom statuses, ignored by clients).
    pub name: String,
    /// The custom status text (custom statuses), or a detail line.
    pub state: Option<String>,
    /// The stream URL (streaming only).
    pub url: Option<String>,
}

impl Activity {
    /// A custom status line, e.g. "💭 thinking…".
    pub fn custom(text: impl Into<String>) -> Self {
        Self {
            kind: ActivityKind::Custom,
            name: "Custom Status".to_owned(),
            state: Some(text.into()),
            url: None,
        }
    }

    /// "Watching {name}".
    pub fn watching(name: impl Into<String>) -> Self {
        Self {
            kind: ActivityKind::Watching,
            name: name.into(),
            state: None,
            url: None,
        }
    }

    /// "Playing {name}".
    pub fn playing(name: impl Into<String>) -> Self {
        Self {
            kind: ActivityKind::Playing,
            name: name.into(),
            state: None,
            url: None,
        }
    }

    /// "Listening to {name}".
    pub fn listening(name: impl Into<String>) -> Self {
        Self {
            kind: ActivityKind::Listening,
            name: name.into(),
            state: None,
            url: None,
        }
    }

    fn to_json(&self) -> Value {
        let mut value = json!({ "name": self.name, "type": self.kind.value() });
        if let Some(state) = &self.state {
            value["state"] = json!(state);
        }
        if let Some(url) = &self.url {
            value["url"] = json!(url);
        }
        value
    }
}

/// The bot's presence: status plus at most one activity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Presence {
    /// Online status.
    pub status: OnlineStatus,
    /// The activity, if any.
    pub activity: Option<Activity>,
}

impl Presence {
    /// Online with no activity.
    pub fn online() -> Self {
        Self {
            status: OnlineStatus::Online,
            activity: None,
        }
    }

    /// The given status with no activity.
    pub fn status(status: OnlineStatus) -> Self {
        Self {
            status,
            activity: None,
        }
    }

    /// Set the activity.
    pub fn with_activity(mut self, activity: Activity) -> Self {
        self.activity = Some(activity);
        self
    }

    /// The `d` body of a Presence Update (also used in Identify).
    pub(crate) fn to_json(&self) -> Value {
        json!({
            "since": null,
            "activities": self.activity.iter().map(Activity::to_json).collect::<Vec<_>>(),
            "status": self.status,
            "afk": false,
        })
    }
}

/// A cloneable handle for sending commands to a running shard.
///
/// Commands queue while the shard is reconnecting and go out once a
/// session is live again. The latest presence is also remembered and sent
/// in every future Identify, so it survives reconnects.
#[derive(Debug, Clone)]
pub struct ShardMessenger {
    pub(crate) sender: mpsc::UnboundedSender<Payload>,
    pub(crate) presence: Arc<Mutex<Option<Presence>>>,
    /// Last heartbeat round trip in milliseconds (0 until the first ack).
    pub(crate) latency: Arc<std::sync::atomic::AtomicU64>,
}

impl ShardMessenger {
    /// The gateway heartbeat round trip (time from a heartbeat to its
    /// ack), or `None` before the first ack.
    pub fn latency(&self) -> Option<std::time::Duration> {
        match self.latency.load(std::sync::atomic::Ordering::Relaxed) {
            0 => None,
            ms => Some(std::time::Duration::from_millis(ms)),
        }
    }

    /// Update the bot's status and activity.
    pub fn update_presence(&self, presence: Presence) -> Result<()> {
        let body = presence.to_json();
        *self.presence.lock().expect("presence lock poisoned") = Some(presence);
        self.queue(Opcode::PresenceUpdate, body)
    }

    /// Join, move, or leave (`channel_id: None`) a voice channel as the bot.
    ///
    /// Axis does not carry voice audio; this only changes the bot's voice
    /// state as seen by others.
    pub fn update_voice_state(
        &self,
        guild_id: GuildId,
        channel_id: Option<ChannelId>,
        self_mute: bool,
        self_deaf: bool,
    ) -> Result<()> {
        self.queue(
            Opcode::VoiceStateUpdate,
            json!({
                "guild_id": guild_id,
                "channel_id": channel_id,
                "self_mute": self_mute,
                "self_deaf": self_deaf,
            }),
        )
    }

    /// Ask for guild members; they arrive as `GUILD_MEMBERS_CHUNK`
    /// dispatches. An empty `query` with `limit: 0` requests every member
    /// (needs the `GUILD_MEMBERS` intent).
    pub fn request_guild_members(
        &self,
        guild_id: GuildId,
        query: impl Into<String>,
        limit: u32,
        nonce: Option<String>,
    ) -> Result<()> {
        let mut body = json!({
            "guild_id": guild_id,
            "query": query.into(),
            "limit": limit,
        });
        if let Some(nonce) = nonce {
            body["nonce"] = json!(nonce);
        }
        self.queue(Opcode::RequestGuildMembers, body)
    }

    /// Ask for specific guild members by ID (up to 100).
    pub fn request_guild_members_by_id(
        &self,
        guild_id: GuildId,
        user_ids: &[UserId],
    ) -> Result<()> {
        self.queue(
            Opcode::RequestGuildMembers,
            json!({
                "guild_id": guild_id,
                "user_ids": user_ids.iter().take(100).collect::<Vec<_>>(),
                "limit": 0,
            }),
        )
    }

    fn queue(&self, op: Opcode, body: Value) -> Result<()> {
        let payload = Payload::command(op, &body)?;
        self.sender.send(payload).map_err(|_| {
            AxisError::Connection(ConnectionError::new("the shard is no longer running"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_status_shape() {
        let presence = Presence::online().with_activity(Activity::custom("thinking"));
        assert_eq!(
            presence.to_json(),
            json!({
                "since": null,
                "activities": [{ "name": "Custom Status", "type": 4, "state": "thinking" }],
                "status": "online",
                "afk": false,
            })
        );
    }

    #[test]
    fn messenger_queues_and_remembers_presence() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let messenger = ShardMessenger {
            sender,
            presence: Arc::new(Mutex::new(None)),
            latency: Arc::default(),
        };
        messenger
            .update_presence(Presence::status(OnlineStatus::Idle))
            .unwrap();
        let payload = receiver.try_recv().unwrap();
        assert_eq!(payload.op, Opcode::PresenceUpdate);
        assert!(messenger.presence.lock().unwrap().is_some());

        messenger
            .update_voice_state(GuildId::new(1), None, false, true)
            .unwrap();
        let payload = receiver.try_recv().unwrap();
        assert_eq!(payload.d.unwrap()["channel_id"], Value::Null);
    }
}
