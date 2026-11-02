//! Gateway protocol types: opcodes, payload envelopes, and the bodies of
//! client-sent commands.
//!
//! Source of truth: the official "Gateway" documentation (opcodes and
//! status codes, heartbeating, identify, resuming).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use axis_core::error::{AxisError, Result, SerializationError};

/// The default gateway URL: version 10, JSON encoding.
pub const DEFAULT_GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";

/// Gateway opcodes, from the documented "Gateway Opcodes" table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Opcode {
    /// Server → client: an event was dispatched.
    Dispatch,
    /// Client → server / server → client: heartbeat, or a request to
    /// heartbeat immediately.
    Heartbeat,
    /// Client → server: start a new session.
    Identify,
    /// Client → server: update the bot's presence (not sent by Axis yet).
    PresenceUpdate,
    /// Client → server: join/move voice channels (not sent by Axis yet).
    VoiceStateUpdate,
    /// Client → server: resume a suspended session.
    Resume,
    /// Server → client: reconnect immediately and resume.
    Reconnect,
    /// Client → server: request guild members (not sent by Axis yet).
    RequestGuildMembers,
    /// Server → client: the session is invalid; `d` says whether it may
    /// resume.
    InvalidSession,
    /// Server → client: first payload on a new connection; carries the
    /// heartbeat interval.
    Hello,
    /// Server → client: reply to a heartbeat.
    HeartbeatAck,
}

impl Opcode {
    pub(crate) const fn from_value(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Dispatch),
            1 => Some(Self::Heartbeat),
            2 => Some(Self::Identify),
            3 => Some(Self::PresenceUpdate),
            4 => Some(Self::VoiceStateUpdate),
            6 => Some(Self::Resume),
            7 => Some(Self::Reconnect),
            8 => Some(Self::RequestGuildMembers),
            9 => Some(Self::InvalidSession),
            10 => Some(Self::Hello),
            11 => Some(Self::HeartbeatAck),
            _ => None,
        }
    }

    pub(crate) const fn value(self) -> u8 {
        match self {
            Self::Dispatch => 0,
            Self::Heartbeat => 1,
            Self::Identify => 2,
            Self::PresenceUpdate => 3,
            Self::VoiceStateUpdate => 4,
            Self::Resume => 6,
            Self::Reconnect => 7,
            Self::RequestGuildMembers => 8,
            Self::InvalidSession => 9,
            Self::Hello => 10,
            Self::HeartbeatAck => 11,
        }
    }
}

/// A gateway payload: the documented envelope `{op, d, s, t}`.
#[derive(Debug, Clone)]
pub(crate) struct Payload {
    pub op: Opcode,
    /// The `d` field. Always present for the payloads Axis handles; `null`
    /// is tolerated as absent.
    pub d: Option<Value>,
    /// The sequence number, present on dispatches.
    pub s: Option<u64>,
    /// The event name, present on dispatches.
    pub t: Option<String>,
}

impl Payload {
    /// Parse a text frame into a payload, rejecting protocol violations
    /// with a [`GatewayError::Protocol`](axis_core::GatewayError).
    pub(crate) fn parse(text: &str) -> Result<Self> {
        #[derive(Deserialize)]
        struct Envelope {
            op: Option<u8>,
            d: Option<Value>,
            s: Option<u64>,
            t: Option<String>,
        }

        let envelope: Envelope = serde_json::from_str(text).map_err(|error| {
            AxisError::Gateway(axis_core::GatewayError::Protocol {
                message: format!("invalid gateway payload: {error}"),
            })
        })?;

        let Some(op_value) = envelope.op else {
            return Err(AxisError::Gateway(axis_core::GatewayError::Protocol {
                message: "gateway payload is missing `op`".to_owned(),
            }));
        };
        let Some(op) = Opcode::from_value(op_value) else {
            return Err(AxisError::Gateway(axis_core::GatewayError::Protocol {
                message: format!("unknown gateway opcode {op_value}"),
            }));
        };

        // A JSON `null` for `d` behaves like an absent field.
        let d = envelope.d.filter(|value| !value.is_null());
        Ok(Self {
            op,
            d,
            s: envelope.s,
            t: envelope.t,
        })
    }

    /// Build a client payload from an opcode and a serializable `d` body.
    pub(crate) fn command<T: Serialize>(op: Opcode, d: &T) -> Result<Self> {
        let value = serde_json::to_value(d)
            .map_err(|error| AxisError::Serialization(SerializationError::Json(error)))?;
        Ok(Self {
            op,
            d: Some(value),
            s: None,
            t: None,
        })
    }

    /// Serialize the payload to a JSON text frame body.
    pub(crate) fn to_json(&self) -> Result<String> {
        #[derive(Serialize)]
        struct Envelope<'a> {
            op: u8,
            #[serde(skip_serializing_if = "Option::is_none")]
            d: Option<&'a Value>,
            #[serde(skip_serializing_if = "Option::is_none")]
            s: Option<u64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            t: Option<&'a str>,
        }

        serde_json::to_string(&Envelope {
            op: self.op.value(),
            d: self.d.as_ref(),
            s: self.s,
            t: self.t.as_deref(),
        })
        .map_err(|error| AxisError::Serialization(SerializationError::Json(error)))
    }

    /// The `heartbeat_interval` of a Hello payload, in milliseconds.
    pub(crate) fn hello_interval(&self) -> Result<u64> {
        let Some(d) = &self.d else {
            return Err(protocol_error("Hello payload is missing `d`"));
        };
        let Some(interval) = d.get("heartbeat_interval").and_then(Value::as_u64) else {
            return Err(protocol_error(
                "Hello payload is missing `heartbeat_interval`",
            ));
        };
        Ok(interval)
    }

    /// The resumability flag of an Invalid Session payload.
    pub(crate) fn invalid_session_resumable(&self) -> Result<bool> {
        let Some(d) = &self.d else {
            return Err(protocol_error("Invalid Session payload is missing `d`"));
        };
        d.as_bool()
            .ok_or_else(|| protocol_error("Invalid Session `d` is not a boolean"))
    }

    /// Extract session identity from a READY dispatch payload.
    pub(crate) fn ready_info(&self) -> Result<ReadyInfo> {
        let Some(d) = &self.d else {
            return Err(protocol_error("READY payload is missing `d`"));
        };
        let session_id = d
            .get("session_id")
            .and_then(Value::as_str)
            .ok_or_else(|| protocol_error("READY payload is missing `session_id`"))?
            .to_owned();
        let resume_gateway_url = d
            .get("resume_gateway_url")
            .and_then(Value::as_str)
            .map(str::to_owned);
        Ok(ReadyInfo {
            session_id,
            resume_gateway_url,
        })
    }
}

fn protocol_error(message: &str) -> AxisError {
    AxisError::Gateway(axis_core::GatewayError::Protocol {
        message: message.to_owned(),
    })
}

/// Session identity carried in a READY dispatch.
#[derive(Debug, Clone)]
pub(crate) struct ReadyInfo {
    pub session_id: String,
    pub resume_gateway_url: Option<String>,
}

/// The `d` body of an Identify payload.
#[derive(Debug, Serialize)]
pub(crate) struct IdentifyBody<'a> {
    pub token: &'a str,
    pub intents: u64,
    pub properties: ConnectionProperties<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shard: Option<[u64; 2]>,
    pub compress: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_threshold: Option<u8>,
}

/// The `properties` object of an Identify payload.
#[derive(Debug, Serialize)]
pub(crate) struct ConnectionProperties<'a> {
    pub os: &'a str,
    pub browser: &'a str,
    pub device: &'a str,
}

/// The `d` body of a Resume payload.
#[derive(Debug, Serialize)]
pub(crate) struct ResumeBody<'a> {
    pub token: &'a str,
    pub session_id: &'a str,
    pub seq: u64,
}

/// Ensure a gateway URL carries the v10 + JSON-encoding query parameters.
///
/// `resume_gateway_url` from READY is a bare `wss://` URL; the docs require
/// resuming "with the same version and encoding as the initial connection".
///
/// A path is also ensured before the query: `ws://host?query` (empty path)
/// produces a malformed HTTP request line during the WebSocket handshake.
pub(crate) fn with_gateway_params(url: &str) -> String {
    let mut url = url.to_owned();
    let after_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(&url);
    if !after_scheme.contains('/') {
        url.push('/');
    }
    if url.contains("v=") && url.contains("encoding=") {
        return url;
    }
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}v=10&encoding=json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_dispatch_envelope() {
        let payload =
            Payload::parse(r#"{"op":0,"d":{"hello":"world"},"s":42,"t":"GUILD_CREATE"}"#).unwrap();
        assert_eq!(payload.op, Opcode::Dispatch);
        assert_eq!(payload.s, Some(42));
        assert_eq!(payload.t.as_deref(), Some("GUILD_CREATE"));
        assert_eq!(payload.d.unwrap(), json!({"hello": "world"}));
    }

    #[test]
    fn parses_null_d_as_absent() {
        let payload = Payload::parse(r#"{"op":11,"d":null,"s":null,"t":null}"#).unwrap();
        assert_eq!(payload.op, Opcode::HeartbeatAck);
        assert_eq!(payload.d, None);
    }

    #[test]
    fn rejects_missing_op_and_unknown_ops() {
        assert!(Payload::parse(r#"{"d":{}}"#).is_err());
        assert!(Payload::parse(r#"{"op":99,"d":{}}"#).is_err());
        assert!(Payload::parse("not json").is_err());
    }

    #[test]
    fn hello_interval_extraction() {
        let payload = Payload::parse(r#"{"op":10,"d":{"heartbeat_interval":41250}}"#).unwrap();
        assert_eq!(payload.hello_interval().unwrap(), 41_250);
        assert!(payload.invalid_session_resumable().is_err());
    }

    #[test]
    fn invalid_session_flag() {
        let payload = Payload::parse(r#"{"op":9,"d":false}"#).unwrap();
        assert!(!payload.invalid_session_resumable().unwrap());
        let payload = Payload::parse(r#"{"op":9,"d":true}"#).unwrap();
        assert!(payload.invalid_session_resumable().unwrap());
    }

    #[test]
    fn ready_info_extraction() {
        let payload = Payload::parse(
            r#"{"op":0,"s":1,"t":"READY","d":{"session_id":"abc","resume_gateway_url":"wss://gateway.discord.gg","user":{}}}"#,
        )
        .unwrap();
        let info = payload.ready_info().unwrap();
        assert_eq!(info.session_id, "abc");
        assert_eq!(
            info.resume_gateway_url.as_deref(),
            Some("wss://gateway.discord.gg")
        );
    }

    #[test]
    fn command_serializes_without_nulls() {
        let body = ResumeBody {
            token: "tok",
            session_id: "sid",
            seq: 1337,
        };
        let payload = Payload::command(Opcode::Resume, &body).unwrap();
        let json = payload.to_json().unwrap();
        // Compare structurally: serde_json sorts object keys in `Value`.
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed,
            json!({ "op": 6, "d": { "token": "tok", "session_id": "sid", "seq": 1337 } })
        );
    }

    #[test]
    fn heartbeat_payload_serializes_sequence() {
        let payload = Payload {
            op: Opcode::Heartbeat,
            d: Some(json!(42)),
            s: None,
            t: None,
        };
        assert_eq!(payload.to_json().unwrap(), r#"{"op":1,"d":42}"#);
        let payload = Payload {
            op: Opcode::Heartbeat,
            d: Some(json!(null)),
            s: None,
            t: None,
        };
        assert_eq!(payload.to_json().unwrap(), r#"{"op":1,"d":null}"#);
    }

    #[test]
    fn gateway_params_appended() {
        // A path is inserted when missing (an empty path + query produces
        // a malformed WebSocket handshake request line).
        assert_eq!(
            with_gateway_params("wss://gateway.discord.gg"),
            "wss://gateway.discord.gg/?v=10&encoding=json"
        );
        assert_eq!(
            with_gateway_params("ws://127.0.0.1:9001"),
            "ws://127.0.0.1:9001/?v=10&encoding=json"
        );
        // Already parameterized URLs are left alone.
        assert_eq!(
            with_gateway_params("wss://g/?v=10&encoding=json"),
            "wss://g/?v=10&encoding=json"
        );
    }
}
