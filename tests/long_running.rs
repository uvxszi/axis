//! Integration tests for the phases working together: gateway events
//! driving REST calls through the `Client`, and a long-running soak over
//! many reconnect/resume cycles with concurrent REST traffic.

use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};

use axis::gateway::{Shard, ShardEvent};
use axis::prelude::*;
use axis::rest::Http;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio_tungstenite::{
    WebSocketStream, accept_async,
    tungstenite::{Message, protocol::CloseFrame},
};

type ServerWs = WebSocketStream<tokio::net::TcpStream>;
type Handler = Box<dyn FnOnce(ServerWs) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

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

fn message_json(id: u64, content: &str) -> String {
    json!({
        "id": id.to_string(),
        "channel_id": "7",
        "author": { "id": "42", "username": "tester", "discriminator": "0" },
        "content": content,
        "timestamp": "2022-09-22T14:00:00.436000+00:00",
        "tts": false, "mention_everyone": false, "mentions": [],
        "mention_roles": [], "attachments": [], "embeds": [],
        "pinned": false, "type": 0
    })
    .to_string()
}

#[tokio::test]
async fn gateway_events_drive_rest_through_the_client() {
    let mut rest = mockito::Server::new_async().await;
    let reply = rest
        .mock("POST", "/channels/7/messages")
        .match_header("authorization", "Bot test-token")
        .match_body(mockito::Matcher::PartialJson(
            json!({ "content": "echo: hi from the gateway" }),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(message_json(999, "echo: hi from the gateway"))
        .create_async()
        .await;

    let mut client = Client::builder(Token::new("test-token".to_owned()))
        .rest_api_base(rest.url())
        // Skip the /gateway fetch; nothing will connect.
        .gateway_url("ws://127.0.0.1:9")
        .build()
        .await
        .expect("client builds");

    client.on_message_create(|ctx, message| async move {
        if message.author.bot.unwrap_or(false) {
            return Ok(());
        }
        ctx.http
            .create_message(message.channel_id)
            .content(format!("echo: {}", message.content))
            .await?;
        Ok(())
    });

    let dispatch = Ok(ShardEvent::Dispatch {
        kind: "MESSAGE_CREATE".to_owned(),
        data: json!({
            "id": "1000", "channel_id": "7",
            "author": { "id": "42", "username": "tester", "discriminator": "0" },
            "content": "hi from the gateway",
            "timestamp": "2022-09-22T14:00:00.436000+00:00",
            "tts": false, "mention_everyone": false, "mentions": [],
            "mention_roles": [], "attachments": [], "embeds": [],
            "pinned": false, "type": 0
        }),
    });

    client
        .run_with(futures_util::stream::iter(vec![dispatch]))
        .await
        .expect("dispatch loop completes");

    reply.assert_async().await;
}

/// A scripted gateway server serving one connection per cycle: the first
/// expects an Identify, the rest expect Resumes carrying the exact last
/// sequence number. Each connection ends with a resumable 4000 close.
async fn spawn_soak_server(
    cycles: usize,
    expected_seq: Arc<AtomicU64>,
    resume_checks: Arc<AtomicUsize>,
) -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("binds");
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let connections = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&connections);

    let mut handlers: Vec<Handler> = Vec::new();
    for cycle in 0..cycles {
        let expected_seq = Arc::clone(&expected_seq);
        let resume_checks = Arc::clone(&resume_checks);
        let url = url.clone();
        handlers.push(Box::new(move |mut ws| {
            Box::pin(async move {
                send_json(&mut ws, hello(50_000)).await;
                if cycle == 0 {
                    let identify = recv_json(&mut ws).await;
                    assert_eq!(identify["op"], 2, "first connection identifies");
                    assert_eq!(identify["d"]["token"], "test-token");
                    send_json(
                        &mut ws,
                        json!({
                            "op": 0, "s": 1, "t": "READY",
                            "d": {
                                "session_id": "soak-session",
                                "resume_gateway_url": url,
                                "user": { "id": "1", "username": "soak-bot", "discriminator": "0" }
                            }
                        }),
                    )
                    .await;
                    expected_seq.store(1, Ordering::SeqCst);
                } else {
                    let resume = recv_json(&mut ws).await;
                    assert_eq!(resume["op"], 6, "cycle {cycle} must resume");
                    assert_eq!(resume["d"]["session_id"], "soak-session");
                    assert_eq!(
                        resume["d"]["seq"],
                        expected_seq.load(Ordering::SeqCst),
                        "resume must carry the exact last sequence"
                    );
                    resume_checks.fetch_add(1, Ordering::SeqCst);
                    let s = expected_seq.load(Ordering::SeqCst) + 1;
                    send_json(
                        &mut ws,
                        json!({ "op": 0, "s": s, "t": "RESUMED", "d": { "_trace": ["soak"] } }),
                    )
                    .await;
                    expected_seq.store(s, Ordering::SeqCst);
                }

                let s = expected_seq.load(Ordering::SeqCst) + 1;
                send_json(
                    &mut ws,
                    json!({
                        "op": 0, "s": s, "t": "MESSAGE_CREATE",
                        "d": { "content": format!("cycle-{cycle}") }
                    }),
                )
                .await;
                expected_seq.store(s, Ordering::SeqCst);

                // Recycle the connection; the session must survive.
                close_ws(&mut ws, 4000, "cycle complete").await;
            })
        }));
    }

    tokio::spawn(async move {
        for handler in handlers {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let Ok(ws) = accept_async(stream).await else {
                continue;
            };
            counter.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(handler(ws));
        }
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let Ok(ws) = accept_async(stream).await else {
                continue;
            };
            counter.fetch_add(1, Ordering::SeqCst);
            drop(ws);
        }
    });

    (url, connections)
}

#[tokio::test]
async fn reconnect_soak_with_concurrent_rest_traffic() {
    const CYCLES: usize = 10;

    let expected_seq = Arc::new(AtomicU64::new(0));
    let resume_checks = Arc::new(AtomicUsize::new(0));
    let (url, connections) = spawn_soak_server(
        CYCLES,
        Arc::clone(&expected_seq),
        Arc::clone(&resume_checks),
    )
    .await;

    // REST traffic runs alongside the gateway the whole time.
    let mut rest = mockito::Server::new_async().await;
    let rest_calls = (CYCLES * 2) / 4;
    let rest_mock = rest
        .mock("POST", "/channels/7/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(message_json(999, "soak"))
        .expect(rest_calls)
        .create_async()
        .await;
    let http = Http::builder(Token::new("test-token".to_owned()))
        .api_base(rest.url())
        .build()
        .unwrap();

    let shard = Shard::builder(
        Token::new("Bot test-token".to_owned()),
        GatewayIntents::GUILDS,
    )
    .gateway_url(url)
    .build()
    .unwrap();
    let mut events = shard.start();

    let mut dispatches = 0usize;
    let mut shutting_down = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        assert!(!remaining.is_zero(), "soak timed out");
        let item = tokio::time::timeout(remaining, events.next_event()).await;
        match item {
            Err(_) => panic!("soak stalled"),
            Ok(None) => break,
            Ok(Some(Ok(ShardEvent::Dispatch { .. }))) => {
                dispatches += 1;
                if dispatches % 4 == 0 {
                    // REST keeps working while the gateway recycles.
                    http.create_message(ChannelId::new(7))
                        .content("soak")
                        .await
                        .expect("REST succeeds during reconnect cycles");
                }
                if dispatches >= CYCLES * 2 && !shutting_down {
                    shard.shutdown();
                    shutting_down = true;
                }
            }
            Ok(Some(Ok(_))) => {}
            Ok(Some(Err(error))) => panic!("shard errored during the soak: {error}"),
        }
    }

    assert_eq!(
        dispatches,
        CYCLES * 2,
        "every cycle's dispatches must arrive exactly once"
    );
    assert_eq!(
        resume_checks.load(Ordering::SeqCst),
        CYCLES - 1,
        "every cycle after the first must resume with the exact sequence"
    );
    assert!(
        connections.load(Ordering::SeqCst) >= CYCLES,
        "the server must have seen at least one connection per cycle"
    );
    rest_mock.assert_async().await;
}

#[tokio::test]
async fn client_lifecycle_identifies_then_shuts_down_gracefully() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let (closed_tx, closed_rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        send_json(&mut ws, hello(50_000)).await;
        let _identify = recv_json(&mut ws).await;
        loop {
            match ws.next().await {
                Some(Ok(Message::Close(frame))) => {
                    let code: u16 = frame.map(|frame| frame.code.into()).unwrap_or(0);
                    assert_eq!(code, 1000, "client shutdown closes with 1000");
                    break;
                }
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                _ => break,
            }
        }
        let _ = closed_tx.send(());
    });

    // A shard built the same way `Client` builds it (gateway_url set, so
    // no /gateway fetch is involved).
    let shard = Shard::builder(Token::new("test-token".to_owned()), GatewayIntents::GUILDS)
        .gateway_url(url)
        .build()
        .unwrap();
    let mut events = shard.start();

    loop {
        match tokio::time::timeout(Duration::from_secs(5), events.next_event()).await {
            Ok(Some(Ok(ShardEvent::Connected))) => break,
            Ok(Some(Ok(_))) => {}
            other => panic!("unexpected event before shutdown: {other:?}"),
        }
    }

    shard.shutdown();
    loop {
        match tokio::time::timeout(Duration::from_secs(5), events.next_event()).await {
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(_) => panic!("stream did not end after shutdown"),
        }
    }
    tokio::time::timeout(Duration::from_secs(2), closed_rx)
        .await
        .expect("server observes the graceful close")
        .expect("handler completes");
}
