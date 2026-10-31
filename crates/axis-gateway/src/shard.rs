//! The shard: public handle, builder, event stream, and the connection
//! supervision loop.

use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures_util::Stream;
use serde_json::json;
use tokio::sync::{mpsc, watch};

use axis_core::{
    AuthError, ConnectionError, GatewayCloseCode, GatewayError, GatewayIntents, Token,
    error::{AxisError, Result},
};

use crate::{
    connection::{self, Incoming, WsSink, WsSource},
    protocol::{self, ConnectionProperties, IdentifyBody, Opcode, Payload, ResumeBody},
    session::Session,
};

/// The client name reported in Identify `properties`.
const CLIENT_NAME: &str = "axis";
/// Close code used when *we* tear down a connection but want Discord to
/// keep the session resumable (per the docs: any code besides 1000/1001).
const RESUME_PRESERVING_CLOSE_CODE: u16 = 4000;
/// Close code for intentional shutdowns, where the session may be dropped.
const SHUTDOWN_CLOSE_CODE: u16 = 1000;
/// How long to wait for the Hello payload after connecting.
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
/// Delay for server-requested reconnects (op 7, resumable op 9): docs say
/// resume immediately; a tiny guard delay avoids tight loops.
const IMMEDIATE_RECONNECT_BASE_MS: u64 = 50;

/// An item yielded by a shard's [`EventStream`].
///
/// Dispatch payloads are forwarded raw; typed event parsing is the job of
/// the `axis-events` crate (Phase 4).
#[derive(Debug, Clone)]
pub enum ShardEvent {
    /// A gateway dispatch (opcode 0).
    Dispatch {
        /// The event name (`t`), e.g. `"MESSAGE_CREATE"`.
        kind: String,
        /// The raw payload (`d`).
        data: serde_json::Value,
    },
    /// An Identify payload was sent on a fresh session. READY follows as a
    /// normal dispatch.
    Connected,
    /// A Resume payload was sent on an existing session. The gateway
    /// replays missed events, then a `RESUMED` dispatch follows.
    Resumed,
    /// The connection was lost or recycled and is being re-established.
    Reconnecting {
        /// The close code the server sent, when the disconnect carried one.
        close_code: Option<u16>,
    },
}

/// The stream of events from a running shard.
///
/// Ends (yields `None`) after the shard shuts down — via
/// [`Shard::shutdown`], dropping the [`Shard`], or a fatal (unrecoverable)
/// gateway close, which is yielded as an `Err` first. Errors are never
/// followed by more items.
///
/// The stream is unbounded internally: if the consumer stops polling, the
/// shard keeps running and events buffer up rather than blocking the
/// heartbeat loop.
pub struct EventStream {
    receiver: mpsc::UnboundedReceiver<Result<ShardEvent, AxisError>>,
}

impl Stream for EventStream {
    type Item = Result<ShardEvent, AxisError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.receiver).poll_recv(cx)
    }
}

impl EventStream {
    /// Wait for the next event.
    pub async fn next_event(&mut self) -> Option<Result<ShardEvent, AxisError>> {
        self.receiver.recv().await
    }
}

/// Configuration for one gateway connection (one shard).
#[derive(Debug)]
struct ShardConfig {
    /// The token in gateway form (no `Bot ` prefix).
    token: String,
    intents: GatewayIntents,
    shard: Option<[u64; 2]>,
    gateway_url: String,
    max_reconnect_attempts: Option<u32>,
}

/// A single gateway connection: identify, heartbeat, resume, and
/// reconnect logic run on a background task.
///
/// Created with [`Shard::builder`] + [`ShardBuilder::build`], started with
/// [`Shard::start`]. Dropping the shard signals a graceful shutdown (close
/// frame 1000, heartbeat loop stops, the event stream ends).
///
/// One `Shard` is one WebSocket connection. Sharding support means running
/// several shards; the configuration shape ([`ShardBuilder::shard`]) and
/// per-connection design keep that possible without redesign.
#[derive(Debug)]
pub struct Shard {
    config: std::sync::Arc<ShardConfig>,
    shutdown: watch::Sender<bool>,
}

impl Shard {
    /// Create a builder for a shard.
    pub fn builder(token: Token, intents: GatewayIntents) -> ShardBuilder {
        ShardBuilder::new(token, intents)
    }

    /// Start the connection task and return the event stream.
    ///
    /// Must be called within a Tokio runtime. Call once per shard; the
    /// stream lives until the shard shuts down or a fatal error occurs.
    pub fn start(&self) -> EventStream {
        let (sender, receiver) = mpsc::unbounded_channel();
        let runner = Runner {
            config: std::sync::Arc::clone(&self.config),
            session: Session::default(),
            events: sender,
        };
        let shutdown = self.shutdown.subscribe();
        tokio::spawn(runner.run(shutdown));
        EventStream { receiver }
    }

    /// Signal a graceful shutdown: the connection is closed with code
    /// 1000, the heartbeat loop stops, and the event stream ends.
    pub fn shutdown(&self) {
        let _ = self.shutdown.send(true);
    }
}

impl Drop for Shard {
    fn drop(&mut self) {
        // Dropping the handle stops the shard; an explicit `shutdown()`
        // does the same earlier.
        let _ = self.shutdown.send(true);
    }
}

/// Builder for a [`Shard`].
#[derive(Debug)]
pub struct ShardBuilder {
    token: Token,
    intents: GatewayIntents,
    shard: Option<[u64; 2]>,
    gateway_url: String,
    max_reconnect_attempts: Option<u32>,
}

impl ShardBuilder {
    fn new(token: Token, intents: GatewayIntents) -> Self {
        Self {
            token,
            intents,
            shard: None,
            gateway_url: protocol::DEFAULT_GATEWAY_URL.to_owned(),
            max_reconnect_attempts: None,
        }
    }

    /// Set this shard's index and the total shard count, per Discord's
    /// sharding scheme. Required only once a bot exceeds 2 500 guilds.
    pub fn shard(mut self, id: u64, total: u64) -> Self {
        self.shard = Some([id, total]);
        self
    }

    /// Override the gateway URL — for tests against a local mock server.
    ///
    /// The `v=10` and `encoding=json` parameters are appended if missing.
    pub fn gateway_url(mut self, url: impl Into<String>) -> Self {
        self.gateway_url = url.into();
        self
    }

    /// Cap how many *consecutive* failed connection attempts are made
    /// before the shard gives up and ends its stream with an error.
    ///
    /// Defaults to unlimited: long-running bots should ride out Discord
    /// outages. The counter resets on every successful identify/resume.
    pub fn max_reconnect_attempts(mut self, max: u32) -> Self {
        self.max_reconnect_attempts = Some(max);
        self
    }

    /// Validate the configuration and build the shard. Never connects.
    pub fn build(self) -> Result<Shard> {
        // The gateway takes the token WITHOUT the "Bot " prefix.
        let raw = self.token.into_inner();
        let token = raw.strip_prefix("Bot ").unwrap_or(&raw).trim().to_owned();
        if token.is_empty() {
            return Err(AxisError::Auth(AuthError::Missing));
        }

        let (shutdown, _) = watch::channel(false);
        Ok(Shard {
            config: std::sync::Arc::new(ShardConfig {
                token,
                intents: self.intents,
                shard: self.shard,
                gateway_url: self.gateway_url,
                max_reconnect_attempts: self.max_reconnect_attempts,
            }),
            shutdown,
        })
    }
}

/// Why a connection ended and must be re-established.
#[derive(Debug)]
enum ReconnectReason {
    /// The gateway sent op 7 (Reconnect).
    ServerRequested,
    /// op 9 (Invalid Session) with `d: true` — the session survives.
    InvalidSessionResumable,
    /// op 9 (Invalid Session) with `d: false` — the session was reset.
    InvalidSessionNotResumable,
    /// The server sent a WebSocket close frame.
    Closed { code: Option<u16> },
    /// The transport failed without a close frame.
    TransportLost,
}

impl ReconnectReason {
    fn close_code(&self) -> Option<u16> {
        match self {
            Self::Closed { code, .. } => *code,
            _ => None,
        }
    }
}

enum ConnectionOutcome {
    Shutdown,
    Fatal(AxisError),
    Reconnect(ReconnectReason),
}

enum PayloadOutcome {
    Continue,
    Reconnect(ReconnectReason),
}

/// The supervision loop for one shard.
struct Runner {
    config: std::sync::Arc<ShardConfig>,
    session: Session,
    events: mpsc::UnboundedSender<Result<ShardEvent, AxisError>>,
}

impl Runner {
    async fn run(mut self, mut shutdown: watch::Receiver<bool>) {
        let mut attempts: u32 = 0;
        loop {
            if *shutdown.borrow() {
                break;
            }
            if let Some(max) = self.config.max_reconnect_attempts {
                if attempts > max {
                    let _ = self
                        .events
                        .send(Err(AxisError::Connection(ConnectionError::new(format!(
                            "gateway reconnect attempts exceeded {max}"
                        )))));
                    break;
                }
            }

            let url =
                protocol::with_gateway_params(self.session.next_url(&self.config.gateway_url));
            let (mut sink, mut source) = match connection::connect(&url).await {
                Ok(halves) => halves,
                Err(error) => {
                    attempts += 1;
                    tracing::warn!(
                        attempt = attempts,
                        %url,
                        error = %error,
                        "gateway connection failed"
                    );
                    if !interruptible_sleep(backoff_delay(attempts), &mut shutdown).await {
                        break;
                    }
                    continue;
                }
            };

            match self
                .run_connection(&mut sink, &mut source, &mut shutdown, &mut attempts)
                .await
            {
                ConnectionOutcome::Shutdown => break,
                ConnectionOutcome::Fatal(error) => {
                    let _ = self.events.send(Err(error));
                    break;
                }
                ConnectionOutcome::Reconnect(reason) => {
                    let close_code = reason.close_code();
                    if self.emit(ShardEvent::Reconnecting { close_code }).is_none() {
                        break;
                    }
                    let delay = match reason {
                        ReconnectReason::ServerRequested
                        | ReconnectReason::InvalidSessionResumable => {
                            Duration::from_millis(IMMEDIATE_RECONNECT_BASE_MS + jitter_ms(100))
                        }
                        _ => backoff_delay(attempts),
                    };
                    if !interruptible_sleep(delay, &mut shutdown).await {
                        break;
                    }
                }
            }
        }
        // Dropping `self.events` ends the stream.
    }

    /// Drive one live connection from Hello through heartbeats until it
    /// ends. Returns what to do next.
    async fn run_connection(
        &mut self,
        sink: &mut WsSink,
        source: &mut WsSource,
        shutdown: &mut watch::Receiver<bool>,
        attempts: &mut u32,
    ) -> ConnectionOutcome {
        // 1. Hello must be the first payload.
        let hello_interval_ms =
            match tokio::time::timeout(HELLO_TIMEOUT, connection::next(source)).await {
                Err(_) => {
                    tracing::warn!("gateway did not send Hello in time");
                    return ConnectionOutcome::Reconnect(ReconnectReason::TransportLost);
                }
                Ok(Err(error)) => {
                    tracing::warn!(%error, "transport error before Hello");
                    return ConnectionOutcome::Reconnect(ReconnectReason::TransportLost);
                }
                Ok(Ok(None)) => {
                    return ConnectionOutcome::Reconnect(ReconnectReason::TransportLost);
                }
                Ok(Ok(Some(Incoming::Closed { code, .. }))) => {
                    tracing::warn!(?code, "gateway closed the connection before Hello");
                    return ConnectionOutcome::Reconnect(ReconnectReason::TransportLost);
                }
                Ok(Ok(Some(Incoming::Payload(payload)))) => match payload.op {
                    Opcode::Hello => match payload.hello_interval() {
                        Ok(interval) => interval,
                        Err(error) => {
                            tracing::error!(%error, "malformed Hello payload");
                            return ConnectionOutcome::Reconnect(ReconnectReason::TransportLost);
                        }
                    },
                    other => {
                        tracing::error!(?other, "expected Hello, got another opcode");
                        return ConnectionOutcome::Reconnect(ReconnectReason::TransportLost);
                    }
                },
            };

        // 2. Identify on a fresh session, Resume when one survives.
        let resuming = self.session.can_resume();
        let command = if resuming {
            Payload::command(
                Opcode::Resume,
                &ResumeBody {
                    token: &self.config.token,
                    session_id: self.session.session_id().unwrap_or_default(),
                    seq: self.session.sequence().unwrap_or(0),
                },
            )
        } else {
            Payload::command(
                Opcode::Identify,
                &IdentifyBody {
                    token: &self.config.token,
                    intents: self.config.intents.bits(),
                    properties: ConnectionProperties {
                        os: std::env::consts::OS,
                        browser: CLIENT_NAME,
                        device: CLIENT_NAME,
                    },
                    shard: self.config.shard,
                    compress: false,
                },
            )
        };
        let command = match command {
            Ok(command) => command,
            Err(error) => return ConnectionOutcome::Fatal(error),
        };
        if let Err(error) = connection::send(sink, &command).await {
            tracing::warn!(%error, "failed to send identify/resume");
            return ConnectionOutcome::Reconnect(ReconnectReason::TransportLost);
        }
        *attempts = 0;
        let lifecycle = if resuming {
            ShardEvent::Resumed
        } else {
            ShardEvent::Connected
        };
        if self.emit(lifecycle).is_none() {
            return ConnectionOutcome::Shutdown;
        }

        // 3. Heartbeats: the first after `interval * jitter` (per the
        //    docs), then one per interval. A missing ack between beats
        //    means the connection is a zombie.
        let interval = Duration::from_millis(hello_interval_ms.max(1));
        let first_delay = Duration::from_secs_f64(interval.as_secs_f64() * jitter_fraction());
        let mut heartbeat_deadline = tokio::time::Instant::now() + first_delay;
        let heartbeat_sleep = tokio::time::sleep_until(heartbeat_deadline);
        tokio::pin!(heartbeat_sleep);
        let mut awaiting_ack = false;

        loop {
            tokio::select! {
                result = shutdown.changed() => {
                    match result {
                        Ok(_) if !*shutdown.borrow() => {}
                        _ => {
                            // Explicit shutdown, or the handle was dropped.
                            let _ = connection::close(
                                sink,
                                SHUTDOWN_CLOSE_CODE,
                                "shutting down",
                            )
                            .await;
                            return ConnectionOutcome::Shutdown;
                        }
                    }
                }
                incoming = connection::next(source) => match incoming {
                    Err(error) => {
                        tracing::warn!(%error, "gateway transport error");
                        return ConnectionOutcome::Reconnect(ReconnectReason::TransportLost);
                    }
                    Ok(None) => {
                        tracing::warn!("gateway connection ended without a close frame");
                        return ConnectionOutcome::Reconnect(ReconnectReason::TransportLost);
                    }
                    Ok(Some(Incoming::Closed { code, reason })) => {
                        return self.on_closed(code, reason);
                    }
                    Ok(Some(Incoming::Payload(payload))) => match payload.op {
                        Opcode::HeartbeatAck => awaiting_ack = false,
                        Opcode::Heartbeat => {
                            // Discord asked for a heartbeat; answer now.
                            let beat = heartbeat_payload(self.session.sequence());
                            if connection::send(sink, &beat).await.is_err() {
                                return ConnectionOutcome::Reconnect(
                                    ReconnectReason::TransportLost,
                                );
                            }
                        }
                        _ => match self.on_payload(payload, sink).await {
                            PayloadOutcome::Continue => {}
                            PayloadOutcome::Reconnect(reason) => {
                                return ConnectionOutcome::Reconnect(reason);
                            }
                        },
                    },
                },
                _ = &mut heartbeat_sleep => {
                    if awaiting_ack {
                        // Zombie connection: terminate with a code that
                        // preserves the session, then resume.
                        tracing::warn!("no heartbeat ack received; terminating zombie connection");
                        let _ = connection::close(
                            sink,
                            RESUME_PRESERVING_CLOSE_CODE,
                            "heartbeat ack timeout",
                        )
                        .await;
                        return ConnectionOutcome::Reconnect(ReconnectReason::TransportLost);
                    }
                    let beat = heartbeat_payload(self.session.sequence());
                    if connection::send(sink, &beat).await.is_err() {
                        return ConnectionOutcome::Reconnect(ReconnectReason::TransportLost);
                    }
                    awaiting_ack = true;
                    heartbeat_deadline = heartbeat_deadline
                        .checked_add(interval)
                        .unwrap_or_else(|| tokio::time::Instant::now() + interval);
                    heartbeat_sleep.as_mut().reset(heartbeat_deadline);
                }
            }
        }
    }

    /// Handle a non-heartbeat payload from the gateway.
    async fn on_payload(&mut self, payload: Payload, sink: &mut WsSink) -> PayloadOutcome {
        match payload.op {
            Opcode::Dispatch => {
                if let Some(sequence) = payload.s {
                    self.session.observe_sequence(sequence);
                }
                let kind = payload.t.clone().unwrap_or_default();
                if kind == "READY" {
                    match payload.ready_info() {
                        Ok(info) => {
                            self.session
                                .adopt_ready(info.session_id, info.resume_gateway_url);
                        }
                        Err(error) => {
                            tracing::error!(
                                %error,
                                "READY payload lacks session identity; resume will not be possible"
                            );
                        }
                    }
                }
                let data = payload.d.unwrap_or(serde_json::Value::Null);
                if self.emit(ShardEvent::Dispatch { kind, data }).is_none() {
                    return PayloadOutcome::Reconnect(ReconnectReason::TransportLost);
                }
                PayloadOutcome::Continue
            }
            Opcode::Reconnect => {
                let _ = connection::close(sink, RESUME_PRESERVING_CLOSE_CODE, "reconnect").await;
                PayloadOutcome::Reconnect(ReconnectReason::ServerRequested)
            }
            Opcode::InvalidSession => match payload.invalid_session_resumable() {
                Ok(true) => {
                    let _ = connection::close(
                        sink,
                        RESUME_PRESERVING_CLOSE_CODE,
                        "invalid session (resumable)",
                    )
                    .await;
                    PayloadOutcome::Reconnect(ReconnectReason::InvalidSessionResumable)
                }
                Ok(false) => {
                    self.session.reset();
                    let _ = connection::close(sink, SHUTDOWN_CLOSE_CODE, "invalid session").await;
                    PayloadOutcome::Reconnect(ReconnectReason::InvalidSessionNotResumable)
                }
                Err(error) => {
                    tracing::error!(%error, "malformed Invalid Session payload");
                    PayloadOutcome::Reconnect(ReconnectReason::TransportLost)
                }
            },
            Opcode::Hello => {
                tracing::warn!("unexpected duplicate Hello payload");
                PayloadOutcome::Continue
            }
            other_op => {
                tracing::warn!(?other_op, "unexpected opcode from the gateway");
                PayloadOutcome::Continue
            }
        }
    }

    /// Decide what a server close frame means for the session.
    fn on_closed(&mut self, code: Option<u16>, reason: Option<String>) -> ConnectionOutcome {
        tracing::debug!(?code, ?reason, "gateway close frame received");
        // The docs: sessions invalidated by 1000/1001 cannot resume.
        if matches!(code, Some(1000 | 1001)) {
            self.session.reset();
            return ConnectionOutcome::Reconnect(ReconnectReason::Closed { code });
        }

        if let Some(code) = code {
            let close = GatewayCloseCode::from_code(code);
            if !close.is_reconnectable() {
                // 4004, 4010-4014: fixing the configuration, not
                // reconnecting, is the only remedy.
                tracing::error!(%close, "gateway closed with a fatal close code");
                return ConnectionOutcome::Fatal(AxisError::Gateway(GatewayError::Closed {
                    code: close,
                    reason,
                }));
            }
            if !close.should_resume() {
                // 4007 / 4009: reconnect allowed, fresh session required.
                self.session.reset();
            }
        }
        ConnectionOutcome::Reconnect(ReconnectReason::Closed { code })
    }

    /// Forward an event; `None` when the consumer is gone.
    fn emit(&self, event: ShardEvent) -> Option<()> {
        self.events.send(Ok(event)).ok()
    }
}

fn heartbeat_payload(sequence: Option<u64>) -> Payload {
    Payload {
        op: Opcode::Heartbeat,
        d: Some(
            sequence
                .map(|seq| json!(seq))
                .unwrap_or(serde_json::Value::Null),
        ),
        s: None,
        t: None,
    }
}

/// Pseudo-random fraction in `0.0..1.0` (non-cryptographic, adequate for
/// heartbeat/reconnect jitter).
fn jitter_fraction() -> f64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.subsec_nanos())
        .unwrap_or(0);
    f64::from(nanos % 1_000) / 1_000.0
}

fn jitter_ms(max: u64) -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.subsec_nanos())
        .unwrap_or(0);
    if max == 0 { 0 } else { u64::from(nanos) % max }
}

/// Exponential backoff for retrying failed connections: 1s, 2s, 4s, ...
/// capped at 30s, plus up to 1s of jitter.
fn backoff_delay(attempts: u32) -> Duration {
    let shift = attempts.min(5);
    let base_ms = 1_000_u64.saturating_mul(1_u64 << shift).min(30_000);
    Duration::from_millis(base_ms + jitter_ms(1_000))
}

/// Sleep for `duration`, returning `false` if a shutdown arrived first.
async fn interruptible_sleep(duration: Duration, shutdown: &mut watch::Receiver<bool>) -> bool {
    if *shutdown.borrow() {
        return false;
    }
    tokio::select! {
        _ = tokio::time::sleep(duration) => true,
        result = shutdown.changed() => match result {
            Ok(_) => !*shutdown.borrow(),
            Err(_) => false, // sender dropped: the handle is gone
        },
    }
}
