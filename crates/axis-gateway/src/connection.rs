//! Thin functional wrapper over the WebSocket transport.
//!
//! The runner keeps the sink and source as separate values so that reads
//! (dispatch handling) and writes (heartbeats, closes) can be selected on
//! concurrently without overlapping borrows.
//!
//! WebSocket-level `Ping` frames are answered by tungstenite itself: it
//! queues a `Pong` when the ping is read, which is transmitted with the
//! next outgoing frame (heartbeats flow at least once per interval, so
//! pongs never sit in the queue for long).

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, protocol::CloseFrame},
};

use axis_core::{
    GatewayError,
    error::{AxisError, Result},
};

use crate::protocol::Payload;

/// The writable half of a gateway WebSocket.
pub(crate) type WsSink =
    futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
/// The readable half of a gateway WebSocket.
pub(crate) type WsSource =
    futures_util::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

/// How long to wait for the WebSocket (and TLS) handshake.
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// An item read from the gateway connection.
#[derive(Debug)]
pub(crate) enum Incoming {
    /// A parsed gateway payload.
    Payload(Payload),
    /// The server closed the connection with (possibly) a close frame.
    Closed {
        code: Option<u16>,
        reason: Option<String>,
    },
}

/// Connect to a gateway URL (must already carry the `v=10` and
/// `encoding=json` parameters) and return the split halves.
pub(crate) async fn connect(url: &str) -> Result<(WsSink, WsSource)> {
    let handshake = tokio::time::timeout(CONNECT_TIMEOUT, tokio_tungstenite::connect_async(url));
    let (stream, _response) = handshake
        .await
        .map_err(|_| connection_error(format!("timed out connecting to {url}")))?
        .map_err(|error| connection_error(format!("failed to connect to {url}: {error}")))?;

    Ok(stream.split())
}

/// Send one payload as a JSON text frame.
pub(crate) async fn send(sink: &mut WsSink, payload: &Payload) -> Result<()> {
    let text = payload.to_json()?;
    sink.send(Message::text(text)).await.map_err(write_error)
}

/// Close the connection with a close code and reason.
///
/// Per Discord's guidance, intentional teardown of a session the server
/// should keep (zombie connections, op 7, resumable op 9) uses a code
/// other than 1000/1001; code 1000 is reserved for real shutdowns, where
/// the session may be dropped.
pub(crate) async fn close(sink: &mut WsSink, code: u16, reason: &str) -> Result<()> {
    let frame = CloseFrame {
        code: code.into(),
        reason: reason.into(),
    };
    sink.send(Message::Close(Some(frame)))
        .await
        .map_err(write_error)?;
    sink.flush().await.map_err(write_error)
}

/// Read the next meaningful item.
///
/// Returns `Ok(None)` when the stream ended without a close frame — an
/// abrupt transport loss.
pub(crate) async fn next(source: &mut WsSource) -> Result<Option<Incoming>> {
    loop {
        match source.next().await {
            Some(Ok(Message::Text(text))) => {
                return Ok(Some(Incoming::Payload(Payload::parse(&text)?)));
            }
            Some(Ok(Message::Binary(bytes))) => {
                // This session uses `encoding=json`; a binary frame is a
                // protocol violation — but it may still be JSON.
                let text = std::str::from_utf8(&bytes).map_err(|_| {
                    AxisError::Gateway(GatewayError::Protocol {
                        message: "binary gateway frame is not valid UTF-8".to_owned(),
                    })
                })?;
                return Ok(Some(Incoming::Payload(Payload::parse(text)?)));
            }
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => {
                // Handled at the protocol layer (see module docs).
            }
            Some(Ok(Message::Close(frame))) => {
                let (code, reason) = match frame {
                    Some(frame) => (Some(u16::from(frame.code)), Some(frame.reason.to_string())),
                    None => (None, None),
                };
                return Ok(Some(Incoming::Closed { code, reason }));
            }
            Some(Ok(Message::Frame(_))) => {}
            Some(Err(error)) => {
                return Err(AxisError::Gateway(GatewayError::Connection {
                    message: "the gateway connection failed".to_owned(),
                    source: Some(Box::new(error)),
                }));
            }
            None => return Ok(None),
        }
    }
}

fn connection_error(message: String) -> AxisError {
    AxisError::Gateway(GatewayError::Connection {
        message,
        source: None,
    })
}

fn write_error(error: tokio_tungstenite::tungstenite::Error) -> AxisError {
    AxisError::Gateway(GatewayError::Connection {
        message: "the gateway socket write failed".to_owned(),
        source: Some(Box::new(error)),
    })
}
