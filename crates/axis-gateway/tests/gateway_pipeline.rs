//! Gateway pipeline tests against a local mock WebSocket server.
//!
//! Each test scripts the server side (Hello → Identify/Resume → dispatches
//! → closes) using the same WebSocket library the client uses, so the
//! reconnect, resume, zombie, and shutdown logic is exercised over real
//! sockets without touching Discord.

use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axis_core::{AxisError, GatewayError, GatewayIntents, Token};
use axis_gateway::{EventStream, Shard, ShardEvent};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio_tungstenite::{
    WebSocketStream, accept_async,
    tungstenite::{Message, protocol::CloseFrame},
};

type ServerWs = WebSocketStream<tokio::net::TcpStream>;
type Handler = Box<dyn FnOnce(ServerWs) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

static INIT_LOGGING: std::sync::Once = std::sync::Once::new();

/// Surface the shard's internal logs (run tests with
/// `RUST_LOG=axis_gateway=trace` for full detail).
fn init_logging() {
    INIT_LOGGING.call_once(|| {
        let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "axis_gateway=debug".to_owned());
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
            .try_init();
    });
}

/// A scripted WebSocket server: one handler per expected connection, in
/// order. Connections beyond the scripted ones are accepted (so reconnect
/// attempts complete the handshake) and counted.
struct TestServer {
    url: String,
    connections: Arc<AtomicUsize>,
}

async fn start_server(handlers: Vec<Handler>) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let connections = spawn_accept_loop(listener, handlers);
    TestServer { url, connections }
}

/// Start a server whose handlers receive the server URL first — for tests
/// where READY must advertise a `resume_gateway_url` that points back at
/// this server.
async fn start_server_with_url(build: impl FnOnce(String) -> Vec<Handler>) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let handlers = build(url.clone());
    let connections = spawn_accept_loop(listener, handlers);
    TestServer { url, connections }
}

fn spawn_accept_loop(listener: TcpListener, handlers: Vec<Handler>) -> Arc<AtomicUsize> {
    let counter = Arc::new(AtomicUsize::new(0));
    let task_counter = Arc::clone(&counter);
    tokio::spawn(async move {
        for handler in handlers {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let Ok(ws) = accept_async(stream).await else {
                continue;
            };
            task_counter.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(handler(ws));
        }
        // Scripted handlers exhausted: accept and drop so unexpected
        // reconnects at least complete the handshake.
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let Ok(ws) = accept_async(stream).await else {
                continue;
            };
            task_counter.fetch_add(1, Ordering::SeqCst);
            drop(ws);
        }
    });
    counter
}

async fn send_json(ws: &mut ServerWs, value: Value) {
    ws.send(Message::text(value.to_string())).await.unwrap();
}

async fn recv_json(ws: &mut ServerWs) -> Value {
    loop {
        match ws.next().await {
            Some(Ok(Message::Text(text))) => {
                return serde_json::from_str(&text).expect("server received valid JSON");
            }
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
            other => panic!("server expected a text frame, got {other:?}"),
        }
    }
}

async fn close_ws(ws: &mut ServerWs, code: u16, reason: &str) {
    ws.send(Message::Close(Some(CloseFrame {
        code: code.into(),
        reason: reason.into(),
    })))
    .await
    .unwrap();
}

fn hello(interval_ms: u64) -> Value {
    json!({ "op": 10, "d": { "heartbeat_interval": interval_ms } })
}

fn ready(seq: u64, session_id: &str, resume_url: &str) -> Value {
    json!({
        "op": 0, "s": seq, "t": "READY",
        "d": {
            "session_id": session_id,
            "resume_gateway_url": resume_url,
            "user": { "id": "1", "username": "axis-test-bot", "discriminator": "0" }
        }
    })
}

async fn expect_event(events: &mut EventStream) -> ShardEvent {
    init_logging();
    let item = tokio::time::timeout(Duration::from_secs(5), events.next_event())
        .await
        .expect("timed out waiting for a shard event");
    item.expect("event stream ended unexpectedly")
        .expect("unexpected stream error")
}

async fn expect_fatal(events: &mut EventStream) -> AxisError {
    init_logging();
    let item = tokio::time::timeout(Duration::from_secs(5), events.next_event())
        .await
        .expect("timed out waiting for the stream error");
    match item {
        Some(Err(error)) => error,
        Some(Ok(event)) => panic!("expected a fatal error, got {event:?}"),
        None => panic!("stream ended without the expected error"),
    }
}

async fn expect_stream_end(events: &mut EventStream) {
    let item = tokio::time::timeout(Duration::from_secs(5), events.next_event())
        .await
        .expect("timed out waiting for the stream to end");
    match item {
        None => {}
        Some(Ok(event)) => panic!("expected stream end, got {event:?}"),
        Some(Err(error)) => panic!("expected clean stream end, got error {error:?}"),
    }
}

fn shard_for(server: &TestServer, intents: GatewayIntents) -> Shard {
    Shard::builder(Token::new("Bot test-token".to_owned()), intents)
        .gateway_url(&server.url)
        .build()
        .expect("shard builds")
}

#[tokio::test]
async fn identifies_with_token_intents_and_properties() {
    init_logging();
    let server = start_server(vec![Box::new(|mut ws| {
        Box::pin(async move {
            send_json(&mut ws, hello(50_000)).await;
            let identify = recv_json(&mut ws).await;
            assert_eq!(identify["op"], 2, "fresh sessions must Identify");
            // The gateway takes the token without the "Bot " prefix.
            assert_eq!(identify["d"]["token"], "test-token");
            assert_eq!(identify["d"]["intents"], 513);
            assert_eq!(identify["d"]["properties"]["os"], std::env::consts::OS);
            assert_eq!(identify["d"]["properties"]["browser"], "axis");
            assert_eq!(identify["d"]["compress"], false);
            send_json(&mut ws, ready(1, "sess-1", "ws://127.0.0.1:9")).await;
        })
    })])
    .await;

    let shard = shard_for(
        &server,
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES,
    );
    let mut events = shard.start();

    let mut saw_connected = false;
    loop {
        match expect_event(&mut events).await {
            ShardEvent::Connected => saw_connected = true,
            ShardEvent::Dispatch { kind, data } if kind == "READY" => {
                assert_eq!(data["session_id"], "sess-1");
                break;
            }
            _ => {}
        }
    }
    assert!(saw_connected, "Connected must precede READY");
}

#[tokio::test]
async fn heartbeats_carry_sequence_and_get_acked() {
    init_logging();
    let server = start_server(vec![Box::new(|mut ws| {
        Box::pin(async move {
            send_json(&mut ws, hello(60)).await;
            let _identify = recv_json(&mut ws).await;

            // First heartbeat: no dispatches seen yet, so `d` is null.
            let beat = recv_json(&mut ws).await;
            assert_eq!(beat["op"], 1);
            assert!(beat["d"].is_null(), "first heartbeat seq must be null");
            send_json(&mut ws, json!({ "op": 11 })).await;

            // READY moves the sequence; the next beat must carry it.
            send_json(&mut ws, ready(1, "sess-1", "ws://127.0.0.1:9")).await;
            let beat = recv_json(&mut ws).await;
            assert_eq!(beat["op"], 1);
            assert_eq!(beat["d"], 1, "heartbeat must carry the last sequence");
            send_json(&mut ws, json!({ "op": 11 })).await;
        })
    })])
    .await;

    let shard = shard_for(&server, GatewayIntents::GUILDS);
    let mut events = shard.start();

    loop {
        match expect_event(&mut events).await {
            ShardEvent::Dispatch { kind, .. } if kind == "READY" => break,
            _ => {}
        }
    }
    // Wait for the second heartbeat round before ending the test.
    tokio::time::sleep(Duration::from_millis(250)).await;
}

#[tokio::test]
async fn dispatches_are_forwarded_raw() {
    init_logging();
    let server = start_server(vec![Box::new(|mut ws| {
        Box::pin(async move {
            send_json(&mut ws, hello(50_000)).await;
            let _identify = recv_json(&mut ws).await;
            send_json(&mut ws, ready(1, "sess-1", "ws://127.0.0.1:9")).await;
            send_json(
                &mut ws,
                json!({ "op": 0, "s": 2, "t": "GUILD_CREATE", "d": { "id": "99" } }),
            )
            .await;
            send_json(
                &mut ws,
                json!({ "op": 0, "s": 3, "t": "MESSAGE_CREATE", "d": { "content": "hi" } }),
            )
            .await;
        })
    })])
    .await;

    let shard = shard_for(&server, GatewayIntents::GUILDS);
    let mut events = shard.start();

    let mut saw_guild_create = false;
    loop {
        match expect_event(&mut events).await {
            ShardEvent::Connected | ShardEvent::Resumed | ShardEvent::Reconnecting { .. } => {}
            ShardEvent::Dispatch { kind, data } => match kind.as_str() {
                "READY" => {}
                "GUILD_CREATE" => {
                    assert_eq!(data["id"], "99");
                    saw_guild_create = true;
                }
                "MESSAGE_CREATE" => {
                    assert_eq!(data["content"], "hi");
                    assert!(saw_guild_create, "events must arrive in order");
                    break;
                }
                other => panic!("unexpected dispatch {other}"),
            },
        }
    }
}

#[tokio::test]
async fn reconnect_opcode_triggers_resume() {
    init_logging();
    let server = start_server_with_url(|url| {
        vec![
            Box::new(move |mut ws| {
                Box::pin(async move {
                    send_json(&mut ws, hello(50_000)).await;
                    let _identify = recv_json(&mut ws).await;
                    send_json(&mut ws, ready(1, "sess-1", &url)).await;
                    // Server-requested recycle.
                    send_json(&mut ws, json!({ "op": 7 })).await;
                })
            }),
            Box::new(|mut ws| {
                Box::pin(async move {
                    send_json(&mut ws, hello(50_000)).await;
                    let resume = recv_json(&mut ws).await;
                    assert_eq!(resume["op"], 6, "op 7 must lead to a Resume");
                    assert_eq!(resume["d"]["token"], "test-token");
                    assert_eq!(resume["d"]["session_id"], "sess-1");
                    assert_eq!(resume["d"]["seq"], 1);
                    send_json(
                        &mut ws,
                        json!({ "op": 0, "s": 2, "t": "RESUMED", "d": { "_trace": ["gateway"] } }),
                    )
                    .await;
                })
            }),
        ]
    })
    .await;

    let shard = shard_for(&server, GatewayIntents::GUILDS);
    let mut events = shard.start();

    let mut saw = (false, false, false, false); // reconnecting, resumed, ready, resumed-dispatch
    loop {
        match expect_event(&mut events).await {
            ShardEvent::Reconnecting { close_code: _ } => saw.0 = true,
            ShardEvent::Resumed => saw.1 = true,
            ShardEvent::Connected => {}
            ShardEvent::Dispatch { kind, .. } => match kind.as_str() {
                "READY" => saw.2 = true,
                "RESUMED" => {
                    saw.3 = true;
                    break;
                }
                _ => {}
            },
        }
    }
    assert!(
        saw.0 && saw.1 && saw.2 && saw.3,
        "lifecycle events out of order: {saw:?}"
    );
}

#[tokio::test]
async fn fatal_close_code_ends_the_stream() {
    init_logging();
    let server = start_server(vec![Box::new(|mut ws| {
        Box::pin(async move {
            send_json(&mut ws, hello(50_000)).await;
            let _identify = recv_json(&mut ws).await;
            close_ws(&mut ws, 4004, "authentication failed").await;
        })
    })])
    .await;

    let shard = shard_for(&server, GatewayIntents::GUILDS);
    let mut events = shard.start();

    // The stream yields Connected, then the fatal error, then ends.
    loop {
        let item = tokio::time::timeout(Duration::from_secs(5), events.next_event())
            .await
            .expect("timed out waiting for events");
        match item {
            Some(Ok(ShardEvent::Connected)) => {}
            Some(Ok(other)) => panic!("unexpected event before the fatal close: {other:?}"),
            Some(Err(error)) => match error {
                AxisError::Gateway(GatewayError::Closed {
                    code: axis_core::GatewayCloseCode::AuthenticationFailed,
                    ..
                }) => break,
                other => panic!("expected AuthenticationFailed close, got {other:?}"),
            },
            None => panic!("stream ended before the fatal error"),
        }
    }
    expect_stream_end(&mut events).await;
    assert_eq!(
        server.connections.load(Ordering::SeqCst),
        1,
        "a fatal close must not trigger a reconnect"
    );
}

#[tokio::test]
async fn close_4007_reconnects_with_fresh_identify() {
    init_logging();
    let server = start_server(vec![
        Box::new(|mut ws| {
            Box::pin(async move {
                send_json(&mut ws, hello(50_000)).await;
                let _identify = recv_json(&mut ws).await;
                send_json(&mut ws, ready(1, "sess-1", "ws://127.0.0.1:9")).await;
                close_ws(&mut ws, 4007, "invalid seq").await;
            })
        }),
        Box::new(|mut ws| {
            Box::pin(async move {
                send_json(&mut ws, hello(50_000)).await;
                let identify = recv_json(&mut ws).await;
                assert_eq!(identify["op"], 2, "4007 requires a fresh session");
                send_json(&mut ws, ready(2, "sess-2", "ws://127.0.0.1:9")).await;
            })
        }),
    ])
    .await;

    let shard = shard_for(&server, GatewayIntents::GUILDS);
    let mut events = shard.start();

    let mut saw_reconnect_with_4007 = false;
    let mut connected_count = 0;
    loop {
        match expect_event(&mut events).await {
            ShardEvent::Reconnecting { close_code } => {
                assert_eq!(close_code, Some(4007));
                saw_reconnect_with_4007 = true;
            }
            ShardEvent::Connected => {
                connected_count += 1;
                if connected_count == 2 {
                    assert!(
                        saw_reconnect_with_4007,
                        "the second connect must follow the 4007 reconnect"
                    );
                    break;
                }
            }
            _ => {}
        }
    }
}

#[tokio::test]
async fn invalid_session_not_resumable_reidentifies() {
    init_logging();
    let server = start_server(vec![
        Box::new(|mut ws| {
            Box::pin(async move {
                send_json(&mut ws, hello(50_000)).await;
                let _identify = recv_json(&mut ws).await;
                send_json(&mut ws, ready(1, "sess-1", "ws://127.0.0.1:9")).await;
                send_json(&mut ws, json!({ "op": 9, "d": false })).await;
            })
        }),
        Box::new(|mut ws| {
            Box::pin(async move {
                send_json(&mut ws, hello(50_000)).await;
                let identify = recv_json(&mut ws).await;
                assert_eq!(identify["op"], 2, "d=false means identify afresh");
                send_json(&mut ws, ready(1, "sess-2", "ws://127.0.0.1:9")).await;
            })
        }),
    ])
    .await;

    let shard = shard_for(&server, GatewayIntents::GUILDS);
    let mut events = shard.start();

    let mut ready_count = 0;
    loop {
        match expect_event(&mut events).await {
            ShardEvent::Dispatch { kind, data } if kind == "READY" => {
                ready_count += 1;
                if ready_count == 2 {
                    assert_eq!(data["session_id"], "sess-2");
                    break;
                }
            }
            _ => {}
        }
    }
}

#[tokio::test]
async fn invalid_session_resumable_resumes() {
    init_logging();
    let server = start_server_with_url(|url| {
        vec![
            Box::new(move |mut ws| {
                Box::pin(async move {
                    send_json(&mut ws, hello(50_000)).await;
                    let _identify = recv_json(&mut ws).await;
                    send_json(&mut ws, ready(3, "sess-1", &url)).await;
                    send_json(&mut ws, json!({ "op": 9, "d": true })).await;
                })
            }),
            Box::new(|mut ws| {
                Box::pin(async move {
                    send_json(&mut ws, hello(50_000)).await;
                    let resume = recv_json(&mut ws).await;
                    assert_eq!(resume["op"], 6, "d=true must resume");
                    assert_eq!(resume["d"]["session_id"], "sess-1");
                    assert_eq!(resume["d"]["seq"], 3);
                })
            }),
        ]
    })
    .await;

    let shard = shard_for(&server, GatewayIntents::GUILDS);
    let mut events = shard.start();

    loop {
        if let ShardEvent::Resumed = expect_event(&mut events).await {
            break;
        }
    }
}

#[tokio::test]
async fn zombie_connection_is_terminated_and_resumed() {
    init_logging();
    let server = start_server_with_url(|url| {
        vec![
            Box::new(move |mut ws| {
                Box::pin(async move {
                    send_json(&mut ws, hello(60)).await;
                    let _identify = recv_json(&mut ws).await;
                    send_json(&mut ws, ready(1, "sess-1", &url)).await;
                    // Read the first heartbeat, deliberately never ack it,
                    // and keep the socket open until the client gives up.
                    let _beat = recv_json(&mut ws).await;
                    loop {
                        match ws.next().await {
                            Some(Ok(Message::Close(frame))) => {
                                let code: u16 = frame.map(|frame| frame.code.into()).unwrap_or(0);
                                assert_ne!(code, 1000, "zombie teardown must preserve the session");
                                break;
                            }
                            Some(Ok(Message::Text(_))) => { /* another heartbeat, no ack */ }
                            Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                            _ => break,
                        }
                    }
                })
            }),
            Box::new(|mut ws| {
                Box::pin(async move {
                    send_json(&mut ws, hello(50_000)).await;
                    let resume = recv_json(&mut ws).await;
                    assert_eq!(resume["op"], 6, "zombie teardown must resume");
                    assert_eq!(resume["d"]["session_id"], "sess-1");
                })
            }),
        ]
    })
    .await;

    let shard = shard_for(&server, GatewayIntents::GUILDS);
    let mut events = shard.start();

    loop {
        if let ShardEvent::Resumed = expect_event(&mut events).await {
            break;
        }
    }
}

#[tokio::test]
async fn transport_drop_resumes() {
    init_logging();
    let server = start_server_with_url(|url| {
        vec![
            Box::new(move |mut ws| {
                Box::pin(async move {
                    send_json(&mut ws, hello(50_000)).await;
                    let _identify = recv_json(&mut ws).await;
                    send_json(&mut ws, ready(1, "sess-1", &url)).await;
                    // Drop the socket without a close frame.
                    drop(ws);
                })
            }),
            Box::new(|mut ws| {
                Box::pin(async move {
                    send_json(&mut ws, hello(50_000)).await;
                    let resume = recv_json(&mut ws).await;
                    assert_eq!(resume["op"], 6, "an abrupt drop must resume");
                    assert_eq!(resume["d"]["session_id"], "sess-1");
                    assert_eq!(resume["d"]["seq"], 1);
                })
            }),
        ]
    })
    .await;

    let shard = shard_for(&server, GatewayIntents::GUILDS);
    let mut events = shard.start();

    loop {
        if let ShardEvent::Resumed = expect_event(&mut events).await {
            break;
        }
    }
}

#[tokio::test]
async fn shutdown_closes_with_1000_and_ends_stream() {
    init_logging();
    let (shutdown_seen_tx, shutdown_seen_rx) = tokio::sync::oneshot::channel();
    let server = start_server(vec![Box::new(move |mut ws| {
        Box::pin(async move {
            send_json(&mut ws, hello(50_000)).await;
            let _identify = recv_json(&mut ws).await;
            // Wait for the client's graceful close.
            loop {
                match ws.next().await {
                    Some(Ok(Message::Close(frame))) => {
                        let code: u16 = frame.map(|frame| frame.code.into()).unwrap_or(0);
                        assert_eq!(code, 1000, "shutdown must close with 1000");
                        break;
                    }
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                    _ => break,
                }
            }
            let _ = shutdown_seen_tx.send(());
        })
    })])
    .await;

    let shard = shard_for(&server, GatewayIntents::GUILDS);
    let mut events = shard.start();

    loop {
        if let ShardEvent::Connected = expect_event(&mut events).await {
            break;
        }
    }

    shard.shutdown();
    expect_stream_end(&mut events).await;
    tokio::time::timeout(Duration::from_secs(2), shutdown_seen_rx)
        .await
        .expect("server must observe the close frame")
        .expect("server handler completes");
}

#[tokio::test]
async fn reconnect_attempts_are_capped_when_configured() {
    init_logging();
    // Nothing listens on port 1: every connect fails immediately.
    let shard = Shard::builder(Token::new("test-token".to_owned()), GatewayIntents::GUILDS)
        .gateway_url("ws://127.0.0.1:1")
        .max_reconnect_attempts(0)
        .build()
        .unwrap();
    let mut events = shard.start();

    let error = expect_fatal(&mut events).await;
    assert!(
        error.to_string().contains("reconnect attempts"),
        "expected the attempts-cap error, got {error:?}"
    );
    expect_stream_end(&mut events).await;
}

#[test]
fn empty_token_is_rejected_at_build() {
    let result = Shard::builder(Token::new("   ".to_owned()), GatewayIntents::GUILDS).build();
    assert!(matches!(result, Err(AxisError::Auth(_))));
}
