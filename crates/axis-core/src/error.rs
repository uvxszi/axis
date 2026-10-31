//! Unified error handling for Axis.
//!
//! All fallible operations in Axis return [`Result`](crate::error::Result)
//! with the top-level [`AxisError`] type. The enum groups errors by
//! subsystem so callers can `match` on what failed, while each variant
//! carries structured context (status codes, route information, retry
//! durations, close codes) instead of opaque strings.
//!
//! ```
//! use axis_core::{AuthError, AxisError};
//!
//! let error = AxisError::from(AuthError::Missing);
//! assert!(matches!(error, AxisError::Auth(AuthError::Missing)));
//! ```

use core::fmt;

use crate::permissions::Permissions;

/// Convenience alias used throughout Axis.
pub type Result<T, E = AxisError> = core::result::Result<T, E>;

/// The top-level error type of the Axis library.
///
/// Errors are grouped by subsystem. Match on the variant to react to a
/// specific failure mode, or treat the value generically (it implements
/// [`std::error::Error`] with source chains preserved).
#[derive(Debug, thiserror::Error)]
pub enum AxisError {
    /// A token was missing or rejected.
    #[error("{0}")]
    Auth(#[from] AuthError),

    /// A transport-level connection failure (DNS, TCP, TLS, timeouts).
    #[error("{0}")]
    Connection(#[from] ConnectionError),

    /// A failure of the Gateway (WebSocket) connection or protocol.
    #[error("{0}")]
    Gateway(#[from] GatewayError),

    /// Discord answered a REST request with an error status.
    #[error("{0}")]
    Http(#[from] HttpError),

    /// A payload or value did not match what Axis expected.
    #[error("{0}")]
    InvalidData(#[from] InvalidDataError),

    /// An invariant inside Axis was violated. This is a bug in Axis.
    #[error("{0}")]
    Internal(#[from] InternalError),

    /// An operation required permissions that were not present.
    #[error("{0}")]
    Permission(#[from] PermissionError),

    /// Discord asked us to slow down (per-route or global rate limit).
    #[error("{0}")]
    RateLimit(#[from] RateLimitError),

    /// Serializing or deserializing a payload failed.
    #[error("{0}")]
    Serialization(#[from] SerializationError),
}

impl AxisError {
    /// Build an [`AxisError::InvalidData`] from a context description.
    pub fn invalid_data(context: impl Into<String>) -> Self {
        Self::InvalidData(InvalidDataError::new(context))
    }

    /// Build an [`AxisError::Internal`] from a description. Internal errors
    /// indicate bugs in Axis itself, not user mistakes.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(InternalError::new(message))
    }

    /// Whether retrying the same operation later may plausibly succeed.
    ///
    /// Connection failures, rate limits, and recoverable gateway closes are
    /// transient; authentication failures, permission problems, and invalid
    /// data are not. Retrying is still subject to rate limits and to the
    /// gateway's own reconnect policy.
    pub fn is_transient(&self) -> bool {
        match self {
            Self::Connection(_) | Self::RateLimit(_) => true,
            Self::Gateway(gateway) => gateway.is_reconnectable(),
            Self::Auth(_)
            | Self::Http(_)
            | Self::InvalidData(_)
            | Self::Internal(_)
            | Self::Permission(_)
            | Self::Serialization(_) => false,
        }
    }
}

/// Errors related to authentication credentials.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// No token was provided where one was required.
    #[error("no token was provided")]
    Missing,

    /// The token was rejected by Discord (HTTP 401 Unauthorized).
    #[error("the token was rejected by Discord (HTTP 401 Unauthorized)")]
    Invalid,
}

/// A Discord REST request failed with a non-success HTTP status.
#[derive(Debug)]
pub struct HttpError {
    /// HTTP status code returned by Discord.
    pub status: u16,
    /// Request path, e.g. `/api/v10/channels/123/messages`.
    pub route: String,
    /// Discord JSON error code, when the response body contained one
    /// (for example `50001` = Missing Access).
    pub code: Option<u32>,
    /// Human-readable message from Discord, when present.
    pub message: Option<String>,
}

impl HttpError {
    /// Build from a response's status and parsed body fields.
    pub fn new(status: u16, route: impl Into<String>) -> Self {
        Self {
            status,
            route: route.into(),
            code: None,
            message: None,
        }
    }

    /// Attach the Discord error code from the response body.
    pub fn with_code(mut self, code: u32) -> Self {
        self.code = Some(code);
        self
    }

    /// Attach the Discord error message from the response body.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Discord API request to {} failed with HTTP {}",
            self.route, self.status
        )?;
        if let Some(code) = self.code {
            write!(f, " (Discord error {code}")?;
            if let Some(message) = &self.message {
                write!(f, ": {message}")?;
            }
            f.write_str(")")?;
        } else if let Some(message) = &self.message {
            write!(f, " ({message})")?;
        }
        Ok(())
    }
}

impl std::error::Error for HttpError {}

/// A rate limit was hit and the request was not performed.
///
/// Axis's REST client resolves most rate limits internally by waiting; this
/// error surfaces only when the client gives up (for example after a global
/// rate limit or too many 429 responses in a row).
#[derive(Debug)]
pub struct RateLimitError {
    /// Whether this is a global rate limit affecting all routes.
    pub global: bool,
    /// How long to wait before retrying, per Discord's `retry_after`.
    pub retry_after: core::time::Duration,
    /// Rate-limit bucket hash, when Discord reported one.
    pub bucket: Option<String>,
    /// Rate-limit scope (`user`, `global`, `shared`), when Discord reported one.
    pub scope: Option<String>,
}

impl RateLimitError {
    /// Build a per-route rate-limit error.
    pub fn route(retry_after: core::time::Duration) -> Self {
        Self {
            global: false,
            retry_after,
            bucket: None,
            scope: None,
        }
    }

    /// Build a global rate-limit error.
    pub fn global(retry_after: core::time::Duration) -> Self {
        Self {
            global: true,
            retry_after,
            bucket: None,
            scope: None,
        }
    }

    /// Whether this is a global rate limit.
    pub const fn is_global(&self) -> bool {
        self.global
    }
}

impl fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.global {
            write!(
                f,
                "global rate limit hit; retry after {:?}",
                self.retry_after
            )
        } else {
            write!(f, "rate limit hit; retry after {:?}", self.retry_after)?;
            if let Some(bucket) = &self.bucket {
                write!(f, " (bucket {bucket})")?;
            }
            Ok(())
        }
    }
}

impl std::error::Error for RateLimitError {}

/// A failure of the Gateway (WebSocket) connection or protocol.
#[derive(Debug)]
pub enum GatewayError {
    /// Connecting to the gateway failed (DNS, TCP, TLS, or handshake).
    Connection {
        /// Description of what failed.
        message: String,
        /// The underlying transport error, when available.
        source: Option<Box<dyn core::error::Error + Send + Sync>>,
    },
    /// A payload from Discord violated the gateway protocol, or an outgoing
    /// payload was rejected.
    Protocol {
        /// Description of the violation.
        message: String,
    },
    /// Discord closed the connection with a close code.
    Closed {
        /// The numeric and symbolic close code.
        code: GatewayCloseCode,
        /// The close reason string Discord sent, when present.
        reason: Option<String>,
    },
}

impl GatewayError {
    /// Whether the connection can be re-established after this error.
    ///
    /// See [`GatewayCloseCode::is_reconnectable`] for close codes; transport
    /// and protocol errors are considered recoverable.
    pub const fn is_reconnectable(&self) -> bool {
        match self {
            Self::Closed { code, .. } => code.is_reconnectable(),
            Self::Connection { .. } | Self::Protocol { .. } => true,
        }
    }
}

impl fmt::Display for GatewayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connection { message, .. } => {
                write!(f, "failed to connect to the gateway: {message}")
            }
            Self::Protocol { message } => write!(f, "gateway protocol error: {message}"),
            Self::Closed { code, reason } => {
                write!(f, "gateway closed by Discord: {code}")?;
                if let Some(reason) = reason {
                    write!(f, " (reason: {reason})")?;
                }
                Ok(())
            }
        }
    }
}

impl core::error::Error for GatewayError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Connection { source, .. } => source.as_deref().map(|e| e as _),
            Self::Protocol { .. } | Self::Closed { .. } => None,
        }
    }
}

/// Why a gateway connection was closed by Discord.
///
/// Values and recovery guidance follow Discord's "Gateway Close Event
/// Codes" table. Codes that the docs mark non-reconnectable "should be
/// considered a signal to stop reconnecting" (fix the underlying problem
/// first — an invalid token or intents configuration will not fix itself).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum GatewayCloseCode {
    /// 4000: We're not sure what went wrong. Try reconnecting?
    #[error("unknown error (4000); try reconnecting")]
    UnknownError,
    /// 4001: An invalid gateway opcode or invalid payload for an opcode was sent.
    #[error("unknown opcode (4001)")]
    UnknownOpcode,
    /// 4002: An invalid payload was sent (too large, or malformed ETF).
    #[error("decode error (4002)")]
    DecodeError,
    /// 4003: A payload was sent before identifying, or the session was invalidated.
    #[error("not authenticated (4003)")]
    NotAuthenticated,
    /// 4004: The token in the identify payload was incorrect.
    #[error("authentication failed (4004); do not reconnect without fixing the token")]
    AuthenticationFailed,
    /// 4005: More than one identify payload was sent.
    #[error("already authenticated (4005)")]
    AlreadyAuthenticated,
    /// 4007: An invalid sequence number was sent when resuming; reconnect and start a new session.
    #[error("invalid sequence (4007); reconnect and start a new session")]
    InvalidSeq,
    /// 4008: Payloads were sent too quickly; the connection is being closed.
    #[error("rate limited (4008)")]
    RateLimited,
    /// 4009: The session timed out; reconnect and start a new one.
    #[error("session timed out (4009); reconnect and start a new session")]
    SessionTimeout,
    /// 4010: An invalid shard was sent when identifying.
    #[error("invalid shard (4010)")]
    InvalidShard,
    /// 4011: The guild count requires sharding to connect.
    #[error("sharding required (4011)")]
    ShardingRequired,
    /// 4012: An invalid gateway version was sent.
    #[error("invalid API version (4012)")]
    InvalidApiVersion,
    /// 4013: An invalid intent value was sent.
    #[error("invalid intents (4013)")]
    InvalidIntents,
    /// 4014: A privileged intent was not enabled or approved for the application.
    #[error("disallowed intents (4014)")]
    DisallowedIntents,
    /// A close code not in the documented table (Discord may add new ones).
    #[error("unknown close code ({0})")]
    Unknown(u16),
}

impl GatewayCloseCode {
    /// Parse a numeric close code received from the gateway.
    pub const fn from_code(code: u16) -> Self {
        match code {
            4000 => Self::UnknownError,
            4001 => Self::UnknownOpcode,
            4002 => Self::DecodeError,
            4003 => Self::NotAuthenticated,
            4004 => Self::AuthenticationFailed,
            4005 => Self::AlreadyAuthenticated,
            4007 => Self::InvalidSeq,
            4008 => Self::RateLimited,
            4009 => Self::SessionTimeout,
            4010 => Self::InvalidShard,
            4011 => Self::ShardingRequired,
            4012 => Self::InvalidApiVersion,
            4013 => Self::InvalidIntents,
            4014 => Self::DisallowedIntents,
            _ => Self::Unknown(code),
        }
    }

    /// The numeric close code.
    pub const fn code(self) -> u16 {
        match self {
            Self::UnknownError => 4000,
            Self::UnknownOpcode => 4001,
            Self::DecodeError => 4002,
            Self::NotAuthenticated => 4003,
            Self::AuthenticationFailed => 4004,
            Self::AlreadyAuthenticated => 4005,
            Self::InvalidSeq => 4007,
            Self::RateLimited => 4008,
            Self::SessionTimeout => 4009,
            Self::InvalidShard => 4010,
            Self::ShardingRequired => 4011,
            Self::InvalidApiVersion => 4012,
            Self::InvalidIntents => 4013,
            Self::DisallowedIntents => 4014,
            Self::Unknown(code) => code,
        }
    }

    /// Whether reconnecting is advised. Codes marked non-reconnectable by
    /// Discord are a signal to stop and fix the application's configuration
    /// (token, shard setup, API version, or intents).
    pub const fn is_reconnectable(self) -> bool {
        !matches!(
            self,
            Self::AuthenticationFailed
                | Self::InvalidShard
                | Self::ShardingRequired
                | Self::InvalidApiVersion
                | Self::InvalidIntents
                | Self::DisallowedIntents
        )
    }

    /// Whether attempting to resume the session is the right recovery.
    ///
    /// Discord explicitly instructs clients to start a new session (a fresh
    /// `Identify`, not a `Resume`) for close codes 4007 and 4009. For the
    /// remaining reconnectable codes a `Resume` attempt is the standard
    /// recovery; if the session is truly gone Discord answers with 4007 and
    /// the client falls back to identifying.
    pub const fn should_resume(self) -> bool {
        self.is_reconnectable() && !matches!(self, Self::InvalidSeq | Self::SessionTimeout)
    }
}

/// Serializing or deserializing a payload failed.
#[derive(Debug, thiserror::Error)]
pub enum SerializationError {
    /// A JSON payload could not be serialized or deserialized.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// A payload was not valid UTF-8.
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] core::str::Utf8Error),
}

/// A payload or value did not match what Axis expected.
#[derive(Debug, thiserror::Error)]
#[error("invalid data: {context}")]
pub struct InvalidDataError {
    /// What was invalid and why.
    pub context: String,
    /// The underlying cause, when available.
    #[source]
    pub source: Option<Box<dyn core::error::Error + Send + Sync>>,
}

impl InvalidDataError {
    /// Build from a context description.
    pub fn new(context: impl Into<String>) -> Self {
        Self {
            context: context.into(),
            source: None,
        }
    }

    /// Attach an underlying cause.
    pub fn with_source(
        context: impl Into<String>,
        source: impl Into<Box<dyn core::error::Error + Send + Sync>>,
    ) -> Self {
        Self {
            context: context.into(),
            source: Some(source.into()),
        }
    }
}

/// A transport-level connection failure (DNS, TCP, TLS, or timeout) for REST
/// or gateway connections.
///
/// Gateway-specific failures use [`GatewayError::Connection`] instead; this
/// type covers HTTP transport problems so callers can distinguish "Discord
/// said no" ([`HttpError`]) from "we never got an answer".
#[derive(Debug, thiserror::Error)]
#[error("connection error: {message}")]
pub struct ConnectionError {
    /// Description of what failed.
    pub message: String,
    /// The underlying transport error, when available.
    #[source]
    pub source: Option<Box<dyn core::error::Error + Send + Sync>>,
}

impl ConnectionError {
    /// Build from a description.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    /// Build from a description and an underlying cause.
    pub fn with_source(
        message: impl Into<String>,
        source: impl Into<Box<dyn core::error::Error + Send + Sync>>,
    ) -> Self {
        Self {
            message: message.into(),
            source: Some(source.into()),
        }
    }
}

/// An internal invariant was violated. This is a bug in Axis, not in the
/// application using it.
#[derive(Debug, thiserror::Error)]
#[error("internal error (this is a bug in Axis, please report it): {message}")]
pub struct InternalError {
    /// Description of the violated invariant.
    pub message: String,
    /// The underlying cause, when available.
    #[source]
    pub source: Option<Box<dyn core::error::Error + Send + Sync>>,
}

impl InternalError {
    /// Build from a description.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }
}

/// An operation required permissions that were not present.
#[derive(Debug)]
pub struct PermissionError {
    /// The permissions that were required but absent.
    pub missing: Permissions,
    /// Optional context, e.g. the route or channel the check applied to.
    pub context: Option<String>,
}

impl PermissionError {
    /// Build from the missing permissions.
    pub fn new(missing: Permissions) -> Self {
        Self {
            missing,
            context: None,
        }
    }

    /// Attach context, e.g. `"creating message in channel 123"`.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

impl fmt::Display for PermissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "missing required permissions: {}", self.missing)?;
        if let Some(context) = &self.context {
            write!(f, " ({context})")?;
        }
        Ok(())
    }
}

impl std::error::Error for PermissionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_display_and_matching() {
        // `AxisError` is transparent for `Display`: the sub-errors carry
        // self-describing messages, so no prefix is duplicated.
        let error = AxisError::from(AuthError::Missing);
        assert_eq!(error.to_string(), "no token was provided");
        assert!(matches!(error, AxisError::Auth(AuthError::Missing)));
    }

    #[test]
    fn constructors() {
        assert_eq!(
            AxisError::invalid_data("boom").to_string(),
            "invalid data: boom"
        );
        assert_eq!(
            AxisError::internal("invariant violated").to_string(),
            "internal error (this is a bug in Axis, please report it): invariant violated"
        );
    }

    #[test]
    fn http_error_display_variants() {
        let bare = HttpError::new(500, "/api/v10/channels/1/messages");
        assert_eq!(
            bare.to_string(),
            "Discord API request to /api/v10/channels/1/messages failed with HTTP 500"
        );

        let detailed = bare.with_code(50_001).with_message("Missing Access");
        assert_eq!(
            detailed.to_string(),
            "Discord API request to /api/v10/channels/1/messages failed with HTTP 500 (Discord error 50001: Missing Access)"
        );

        let message_only = HttpError::new(503, "/x").with_message("service unavailable");
        assert_eq!(
            message_only.to_string(),
            "Discord API request to /x failed with HTTP 503 (service unavailable)"
        );
    }

    #[test]
    fn rate_limit_error_display() {
        let route = RateLimitError::route(core::time::Duration::from_millis(1200));
        assert_eq!(route.to_string(), "rate limit hit; retry after 1.2s");
        let global = RateLimitError::global(core::time::Duration::from_secs(2));
        assert!(global.is_global());
        assert_eq!(global.to_string(), "global rate limit hit; retry after 2s");
    }

    #[test]
    fn gateway_error_source_chain() {
        let source = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        let error = GatewayError::Connection {
            message: "tcp connect failed".into(),
            source: Some(Box::new(source)),
        };
        let top = AxisError::Gateway(error);

        // Walk the chain: AxisError -> GatewayError -> io::Error.
        let first = std::error::Error::source(&top).expect("gateway error is the first source");
        assert!(first.to_string().contains("tcp connect failed"));
        let second = first
            .source()
            .expect("transport error is the second source");
        assert_eq!(second.to_string(), "refused");
        assert!(second.source().is_none());
    }

    #[test]
    fn close_code_table() {
        assert_eq!(GatewayCloseCode::from_code(4000).code(), 4000);
        assert_eq!(
            GatewayCloseCode::from_code(4004),
            GatewayCloseCode::AuthenticationFailed
        );
        assert_eq!(GatewayCloseCode::from_code(4014).code(), 4014);
        assert_eq!(
            GatewayCloseCode::from_code(9999),
            GatewayCloseCode::Unknown(9999)
        );
        assert_eq!(GatewayCloseCode::Unknown(9999).code(), 9999);
    }

    #[test]
    fn close_code_recovery_semantics() {
        // Non-fatal: reconnect, try to resume.
        assert!(GatewayCloseCode::UnknownError.is_reconnectable());
        assert!(GatewayCloseCode::UnknownError.should_resume());

        // Explicit "start a new session" per the docs.
        assert!(GatewayCloseCode::InvalidSeq.is_reconnectable());
        assert!(!GatewayCloseCode::InvalidSeq.should_resume());
        assert!(GatewayCloseCode::SessionTimeout.is_reconnectable());
        assert!(!GatewayCloseCode::SessionTimeout.should_resume());

        // Fatal configuration errors: stop reconnecting entirely.
        for fatal in [
            GatewayCloseCode::AuthenticationFailed,
            GatewayCloseCode::InvalidShard,
            GatewayCloseCode::ShardingRequired,
            GatewayCloseCode::InvalidApiVersion,
            GatewayCloseCode::InvalidIntents,
            GatewayCloseCode::DisallowedIntents,
        ] {
            assert!(!fatal.is_reconnectable(), "{fatal:?} must not reconnect");
            assert!(!fatal.should_resume(), "{fatal:?} must not resume");
        }

        // Undocumented codes: assume transient.
        assert!(GatewayCloseCode::Unknown(4242).is_reconnectable());
    }

    #[test]
    fn transient_classification() {
        assert!(AxisError::from(ConnectionError::new("timeout")).is_transient());
        assert!(
            AxisError::from(RateLimitError::route(core::time::Duration::from_secs(1)))
                .is_transient()
        );
        assert!(
            AxisError::Gateway(GatewayError::Closed {
                code: GatewayCloseCode::SessionTimeout,
                reason: None,
            })
            .is_transient()
        );
        assert!(!AxisError::from(AuthError::Invalid).is_transient());
        assert!(!AxisError::invalid_data("x").is_transient());
        assert!(
            !AxisError::Gateway(GatewayError::Closed {
                code: GatewayCloseCode::AuthenticationFailed,
                reason: None,
            })
            .is_transient()
        );
    }

    #[test]
    fn permission_error_display() {
        let error = PermissionError::new(Permissions::KICK_MEMBERS | Permissions::BAN_MEMBERS)
            .with_context("kicking user 123");
        assert_eq!(
            error.to_string(),
            "missing required permissions: KICK_MEMBERS, BAN_MEMBERS (kicking user 123)"
        );
    }

    #[test]
    fn serialization_error_wraps_json() {
        let json_error = serde_json::from_str::<crate::id::UserId>("\"nope\"")
            .err()
            .unwrap();
        let serialization_error = SerializationError::from(json_error);
        let error = AxisError::from(serialization_error);
        assert!(matches!(
            error,
            AxisError::Serialization(SerializationError::Json(_))
        ));
        assert!(!error.is_transient());
    }
}
