//! Pipeline tests against a local mock HTTP server (mockito).
//!
//! These exercise the real request path — authentication headers, request
//! bodies, query strings, rate-limit handling, retries, and error mapping
//! — without touching Discord.

use std::time::{Duration, Instant};

use axis_core::{
    AuthError, AxisError, ChannelId, EmojiId, GenericId, GuildId, MessageId, Permissions, Token,
    UserId,
};
use axis_model::{ChannelType, Emoji, OverwriteType};
use axis_rest::Http;
use mockito::{Matcher, Server};
use serde_json::json;

fn http_for(server_url: &str) -> Http {
    Http::builder(Token::new("test-token".to_owned()))
        .api_base(server_url)
        .build()
        .expect("test client builds")
}

/// A minimal but valid message payload.
fn message_json(id: u64, content: &str) -> String {
    json!({
        "id": id.to_string(),
        "channel_id": "1",
        "author": { "id": "42", "username": "tester", "discriminator": "0" },
        "content": content,
        "timestamp": "2022-09-22T14:00:00.436000+00:00",
        "tts": false,
        "mention_everyone": false,
        "mentions": [],
        "mention_roles": [],
        "attachments": [],
        "embeds": [],
        "pinned": false,
        "type": 0
    })
    .to_string()
}

#[tokio::test]
async fn create_message_sends_body_and_parses_response() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/channels/1/messages")
        .match_header("authorization", "Bot test-token")
        .match_header("content-type", "application/json")
        .match_body(Matcher::PartialJson(
            json!({ "content": "hello from axis" }),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(message_json(777, "hello from axis"))
        .create_async()
        .await;

    let http = http_for(&server.url());
    let message = http
        .create_message(ChannelId::new(1))
        .content("hello from axis")
        .await
        .expect("message creation succeeds");

    assert_eq!(message.id.get(), 777);
    assert_eq!(message.content, "hello from axis");
    assert_eq!(message.channel_id.get(), 1);
    assert_eq!(message.author.username, "tester");
    mock.assert_async().await;
}

#[tokio::test]
async fn raw_token_gets_bot_prefix() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/guilds/5")
        .match_header("authorization", "Bot test-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "id": "5", "name": "g", "owner_id": "1", "afk_timeout": 300,
                "verification_level": 0, "default_message_notifications": 0,
                "explicit_content_filter": 0, "roles": [], "emojis": [],
                "features": [], "mfa_level": 0, "premium_tier": 0,
                "preferred_locale": "en-US", "nsfw_level": 0,
                "premium_progress_bar_enabled": false
            })
            .to_string(),
        )
        .create_async()
        .await;

    let http = http_for(&server.url());
    let guild = http.get_guild(GuildId::new(5)).await.unwrap();
    assert_eq!(guild.id.get(), 5);
    mock.assert_async().await;
}

#[tokio::test]
async fn prefixed_token_is_not_double_prefixed() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/channels/1")
        .match_header("authorization", "Bot test-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json!({ "id": "1", "type": 0 }).to_string())
        .create_async()
        .await;

    let http = Http::builder(Token::new("Bot test-token".to_owned()))
        .api_base(server.url())
        .build()
        .unwrap();
    let channel = http.get_channel(ChannelId::new(1)).await.unwrap();
    assert_eq!(channel.r#type, ChannelType::GuildText);
    mock.assert_async().await;
}

#[tokio::test]
async fn unauthorized_maps_to_auth_error() {
    let mut server = Server::new_async().await;
    server
        .mock("GET", "/guilds/5")
        .with_status(401)
        .with_body(json!({ "message": "401: Unauthorized", "code": 0 }).to_string())
        .create_async()
        .await;

    let http = http_for(&server.url());
    let error = http
        .get_guild(GuildId::new(5))
        .await
        .expect_err("401 must fail");
    assert!(matches!(error, AxisError::Auth(AuthError::Invalid)));
}

#[tokio::test]
async fn error_body_is_parsed_into_http_error() {
    let mut server = Server::new_async().await;
    server
        .mock("GET", "/channels/1")
        .with_status(404)
        .with_body(json!({ "message": "Unknown Channel", "code": 10_007 }).to_string())
        .create_async()
        .await;

    let http = http_for(&server.url());
    let error = http
        .get_channel(ChannelId::new(1))
        .await
        .expect_err("404 must fail");
    let AxisError::Http(http_error) = error else {
        panic!("expected AxisError::Http, got {error:?}")
    };
    assert_eq!(http_error.status, 404);
    assert_eq!(http_error.code, Some(10_007));
    assert_eq!(http_error.message.as_deref(), Some("Unknown Channel"));
    assert_eq!(http_error.route, "/channels/1");
}

#[tokio::test]
async fn rate_limited_request_waits_then_succeeds() {
    let mut server = Server::new_async().await;
    // Mockito prefers the first mock that has not reached its expected hit
    // count, so the 429 mock is created first and matches the initial
    // request; the 200 mock takes over on the retry.
    let limited = server
        .mock("GET", "/channels/1/messages/2")
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "message": "You are being rate limited.",
                "retry_after": 0.2,
                "global": false,
                "code": 0
            })
            .to_string(),
        )
        .expect(1)
        .create_async()
        .await;
    let ok = server
        .mock("GET", "/channels/1/messages/2")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(message_json(2, "eventually"))
        .expect(1)
        .create_async()
        .await;

    let http = http_for(&server.url());
    let start = Instant::now();
    let message = http
        .get_message(ChannelId::new(1), MessageId::new(2))
        .await
        .unwrap();
    let elapsed = start.elapsed();

    assert_eq!(message.content, "eventually");
    assert!(
        elapsed >= Duration::from_millis(180),
        "client must honor retry_after, took {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(3),
        "waited too long: {elapsed:?}"
    );
    limited.assert_async().await;
    ok.assert_async().await;
}

#[tokio::test]
async fn rate_limit_retries_are_bounded() {
    let mut server = Server::new_async().await;
    let limited = server
        .mock("GET", "/channels/1/messages/2")
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_body(
            json!({ "message": "rate limited", "retry_after": 0.05, "global": false }).to_string(),
        )
        .expect(2)
        .create_async()
        .await;

    let http = Http::builder(Token::new("test-token".to_owned()))
        .api_base(server.url())
        .max_rate_limit_retries(1)
        .build()
        .unwrap();

    let error = http
        .get_message(ChannelId::new(1), MessageId::new(2))
        .await
        .expect_err("exhausted retries must fail");
    let AxisError::RateLimit(rate_limit) = error else {
        panic!("expected AxisError::RateLimit, got {error:?}")
    };
    assert!(!rate_limit.is_global());
    assert_eq!(
        rate_limit.retry_after,
        Duration::from_millis(50),
        "retry_after must come from the body"
    );
    limited.assert_async().await;
}

#[tokio::test]
async fn global_rate_limit_is_flagged() {
    let mut server = Server::new_async().await;
    server
        .mock("GET", "/channels/1/messages/2")
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_body(
            json!({ "message": "global rate limited", "retry_after": 0.05, "global": true })
                .to_string(),
        )
        .expect(2)
        .create_async()
        .await;

    let http = Http::builder(Token::new("test-token".to_owned()))
        .api_base(server.url())
        .max_rate_limit_retries(1)
        .build()
        .unwrap();

    let error = http
        .get_message(ChannelId::new(1), MessageId::new(2))
        .await
        .expect_err("exhausted retries must fail");
    let AxisError::RateLimit(rate_limit) = error else {
        panic!("expected AxisError::RateLimit, got {error:?}")
    };
    assert!(rate_limit.is_global());
}

#[tokio::test]
async fn exhausted_bucket_waits_before_second_request() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/channels/1/messages/2")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(message_json(2, "one"))
        // Response headers: bucket exhausted, resets in 300ms.
        .with_header("x-ratelimit-bucket", "abc123")
        .with_header("x-ratelimit-remaining", "0")
        .with_header("x-ratelimit-limit", "1")
        .with_header("x-ratelimit-reset-after", "0.3")
        .expect(2)
        .create_async()
        .await;

    let http = http_for(&server.url());
    let start = Instant::now();
    let first = http
        .get_message(ChannelId::new(1), MessageId::new(2))
        .await
        .unwrap();
    let second = http
        .get_message(ChannelId::new(1), MessageId::new(2))
        .await
        .unwrap();
    let elapsed = start.elapsed();

    assert_eq!(first.content, "one");
    assert_eq!(second.content, "one");
    assert!(
        elapsed >= Duration::from_millis(280),
        "second request must wait for the bucket reset, took {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(3),
        "waited too long: {elapsed:?}"
    );
    mock.assert_async().await;
}

#[tokio::test]
async fn server_errors_are_retried_on_get() {
    let mut server = Server::new_async().await;
    // Created first so it matches the initial request; the success mock
    // takes over on the retry.
    let failure = server
        .mock("GET", "/guilds/5")
        .with_status(502)
        .expect(1)
        .create_async()
        .await;
    let ok = server
        .mock("GET", "/guilds/5")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "id": "5", "name": "g", "owner_id": "1", "afk_timeout": 300,
                "verification_level": 0, "default_message_notifications": 0,
                "explicit_content_filter": 0, "roles": [], "emojis": [],
                "features": [], "mfa_level": 0, "premium_tier": 0,
                "preferred_locale": "en-US", "nsfw_level": 0,
                "premium_progress_bar_enabled": false
            })
            .to_string(),
        )
        .expect(1)
        .create_async()
        .await;

    let http = http_for(&server.url());
    let guild = http
        .get_guild(GuildId::new(5))
        .await
        .expect("retry succeeds");
    assert_eq!(guild.name, "g");
    failure.assert_async().await;
    ok.assert_async().await;
}

#[tokio::test]
async fn server_errors_are_not_retried_on_post() {
    let mut server = Server::new_async().await;
    let failure = server
        .mock("POST", "/channels/1/messages")
        .with_status(500)
        .expect(1)
        .create_async()
        .await;

    let http = http_for(&server.url());
    let error = http
        .create_message(ChannelId::new(1))
        .content("should not be duplicated")
        .await
        .expect_err("POST must not be retried after 500");
    let AxisError::Http(http_error) = error else {
        panic!("expected AxisError::Http, got {error:?}")
    };
    assert_eq!(http_error.status, 500);
    // Exactly one request happened: no duplicate-send risk.
    failure.assert_async().await;
}

#[tokio::test]
async fn delete_message_tolerates_204() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("DELETE", "/channels/1/messages/2")
        .match_header("authorization", "Bot test-token")
        .with_status(204)
        .create_async()
        .await;

    let http = http_for(&server.url());
    http.delete_message(ChannelId::new(1), MessageId::new(2))
        .await
        .expect("204 deletion succeeds");
    mock.assert_async().await;
}

#[tokio::test]
async fn list_messages_builds_query_string() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/channels/1/messages")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("limit".into(), "50".into()),
            Matcher::UrlEncoded("before".into(), "999".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!([serde_json::from_str::<serde_json::Value>(&message_json(10, "m")).unwrap()])
                .to_string(),
        )
        .create_async()
        .await;

    let http = http_for(&server.url());
    let messages = http
        .get_messages(ChannelId::new(1))
        .limit(50)
        .before(MessageId::new(999))
        .await
        .unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id.get(), 10);
    mock.assert_async().await;
}

#[tokio::test]
async fn unicode_reaction_is_percent_encoded() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("PUT", "/channels/1/messages/2/reactions/%F0%9F%94%A5/@me")
        .with_status(204)
        .create_async()
        .await;

    let http = http_for(&server.url());
    let emoji = Emoji {
        id: None,
        name: Some("🔥".to_owned()),
        animated: None,
    };
    http.add_reaction(ChannelId::new(1), MessageId::new(2), emoji)
        .await
        .expect("reaction succeeds");
    mock.assert_async().await;
}

#[tokio::test]
async fn custom_reaction_is_encoded_with_colon() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock(
            "DELETE",
            "/channels/1/messages/2/reactions/ultra%3A417719834621489152/42",
        )
        .with_status(204)
        .create_async()
        .await;

    let http = http_for(&server.url());
    let emoji = Emoji {
        id: Some(EmojiId::new(417_719_834_621_489_152)),
        name: Some("ultra".to_owned()),
        animated: Some(false),
    };
    http.delete_user_reaction(ChannelId::new(1), MessageId::new(2), emoji, UserId::new(42))
        .await
        .expect("reaction deletion succeeds");
    mock.assert_async().await;
}

#[tokio::test]
async fn bulk_delete_sends_id_array() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/channels/1/messages/bulk-delete")
        .match_body(Matcher::PartialJson(json!({
            "messages": ["10", "20", "30"]
        })))
        .with_status(204)
        .create_async()
        .await;

    let http = http_for(&server.url());
    http.bulk_delete_messages(
        ChannelId::new(1),
        vec![MessageId::new(10), MessageId::new(20), MessageId::new(30)],
    )
    .await
    .expect("bulk delete succeeds");
    mock.assert_async().await;
}

#[tokio::test]
async fn bulk_delete_validates_count() {
    let http = Http::builder(Token::new("test-token".to_owned()))
        .api_base("http://127.0.0.1:9")
        .build()
        .unwrap();
    let error = http
        .bulk_delete_messages(ChannelId::new(1), vec![MessageId::new(1)])
        .await
        .expect_err("one message cannot be bulk-deleted");
    assert!(error.to_string().contains("2 and 100"));
}

#[tokio::test]
async fn ban_member_sends_delete_message_seconds() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("PUT", "/guilds/5/bans/42")
        .match_body(Matcher::PartialJson(
            json!({ "delete_message_seconds": 86_400 }),
        ))
        .with_status(204)
        .create_async()
        .await;

    let http = http_for(&server.url());
    http.ban_member(GuildId::new(5), UserId::new(42))
        .delete_message_seconds(86_400)
        .await
        .expect("ban succeeds");
    mock.assert_async().await;
}

#[tokio::test]
async fn clear_timeout_sends_null() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("PATCH", "/guilds/5/members/42")
        .match_body(Matcher::PartialJson(
            json!({ "communication_disabled_until": serde_json::Value::Null }),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "roles": [], "deaf": false, "mute": false,
                "joined_at": "2020-01-01T00:00:00.000000+00:00"
            })
            .to_string(),
        )
        .create_async()
        .await;

    let http = http_for(&server.url());
    let member = http
        .edit_member(GuildId::new(5), UserId::new(42))
        .clear_timeout()
        .await
        .expect("member edit succeeds");
    assert!(member.communication_disabled_until.is_none());
    mock.assert_async().await;
}

#[tokio::test]
async fn permission_overwrite_sends_string_bitsets() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("PUT", "/channels/1/permissions/999")
        .match_body(Matcher::PartialJson(json!({
            "type": 0,
            "allow": "1024",
            "deny": "2048"
        })))
        .with_status(204)
        .create_async()
        .await;

    let http = http_for(&server.url());
    http.edit_channel_permission(ChannelId::new(1), GenericId::new(999), OverwriteType::Role)
        .allow(Permissions::VIEW_CHANNEL)
        .deny(Permissions::SEND_MESSAGES)
        .await
        .expect("overwrite edit succeeds");
    mock.assert_async().await;
}

#[tokio::test]
async fn create_role_serializes_permissions_as_string() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/guilds/5/roles")
        .match_body(Matcher::PartialJson(json!({
            "name": "mods",
            "permissions": "6",
            "hoist": true
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "id": "600", "name": "mods", "color": 0,
                "colors": { "primary_color": 0 }, "hoist": true, "position": 1,
                "permissions": "6", "managed": false, "mentionable": false
            })
            .to_string(),
        )
        .create_async()
        .await;

    let http = http_for(&server.url());
    let role = http
        .create_role(GuildId::new(5))
        .name("mods")
        .permissions(Permissions::KICK_MEMBERS | Permissions::BAN_MEMBERS)
        .hoist(true)
        .await
        .expect("role creation succeeds");
    assert_eq!(role.id.get(), 600);
    mock.assert_async().await;
}

#[tokio::test]
async fn kick_member_hits_correct_route() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("DELETE", "/guilds/5/members/42")
        .with_status(204)
        .create_async()
        .await;

    let http = http_for(&server.url());
    http.kick_member(GuildId::new(5), UserId::new(42))
        .await
        .expect("kick succeeds");
    mock.assert_async().await;
}

#[tokio::test]
async fn concurrent_creates_on_same_channel_all_succeed() {
    // The limiter and pipeline must be safe under concurrent requests:
    // five creates fire at once against an unlimited mock, and every one
    // must succeed.
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/channels/1/messages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(message_json(1, "concurrent"))
        .expect(5)
        .create_async()
        .await;

    let http = std::sync::Arc::new(http_for(&server.url()));
    let handles: Vec<_> = (0..5)
        .map(|_| {
            let http = std::sync::Arc::clone(&http);
            tokio::spawn(async move {
                http.create_message(ChannelId::new(1))
                    .content("concurrent")
                    .await
            })
        })
        .collect();
    for handle in handles {
        handle
            .await
            .expect("task joins")
            .expect("all concurrent creates succeed");
    }
    mock.assert_async().await;
}
