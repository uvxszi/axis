//! Tests for event parsing and dispatch, driven the way a bot drives it:
//! raw gateway dispatches in, handler calls out.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use axis_core::Result;
use axis_events::{Dispatcher, Event};
use axis_gateway::ShardEvent;
use futures_util::stream;
use serde_json::{Value, json};

fn parse_event(kind: &str, data: Value) -> Result<Event> {
    Event::parse(kind, data)
}

#[test]
fn parses_ready() {
    let event = parse_event(
        "READY",
        json!({
            "v": 10,
            "user": { "id": "80351110224678912", "username": "axis-bot", "discriminator": "0", "bot": true },
            "guilds": [ { "id": "290926798999268355", "unavailable": true } ],
            "session_id": "sess-1",
            "resume_gateway_url": "wss://gateway.discord.gg",
            "application": { "id": "80351110224678912", "flags": 8192 }
        }),
    )
    .unwrap();

    let Event::Ready(ready) = event else {
        panic!("expected Ready, got {event:?}")
    };
    assert_eq!(ready.session_id, "sess-1");
    assert_eq!(ready.user.username, "axis-bot");
    assert_eq!(ready.guilds.len(), 1);
}

#[test]
fn parses_message_lifecycle() {
    let author = json!({ "id": "42", "username": "tester", "discriminator": "0" });
    let base = json!({
        "id": "1000", "channel_id": "7", "author": author,
        "content": "hello", "timestamp": "2022-09-22T14:00:00.436000+00:00",
        "tts": false, "mention_everyone": false, "mentions": [],
        "mention_roles": [], "attachments": [], "embeds": [], "pinned": false, "type": 0
    });

    let event = parse_event("MESSAGE_CREATE", base.clone()).unwrap();
    let Event::MessageCreate(message) = event else {
        panic!("expected MessageCreate, got {event:?}")
    };
    assert_eq!(message.content, "hello");
    assert_eq!(message.id.get(), 1000);

    // Updates are partial: only a few fields, no author requirement.
    let event = parse_event(
        "MESSAGE_UPDATE",
        json!({ "id": "1000", "channel_id": "7", "content": "edited" }),
    )
    .unwrap();
    let Event::MessageUpdate(partial) = event else {
        panic!("expected MessageUpdate, got {event:?}")
    };
    assert_eq!(partial.content.as_deref(), Some("edited"));
    assert_eq!(partial.author, None);

    let event = parse_event(
        "MESSAGE_DELETE",
        json!({ "id": "1000", "channel_id": "7", "guild_id": "9" }),
    )
    .unwrap();
    let Event::MessageDelete(delete) = event else {
        panic!("expected MessageDelete, got {event:?}")
    };
    assert_eq!(delete.channel_id.get(), 7);

    let event = parse_event(
        "MESSAGE_DELETE_BULK",
        json!({ "ids": ["1000", "1001"], "channel_id": "7" }),
    )
    .unwrap();
    let Event::MessageDeleteBulk(bulk) = event else {
        panic!("expected MessageDeleteBulk, got {event:?}")
    };
    assert_eq!(bulk.ids.len(), 2);
}

#[test]
fn parses_reaction_events() {
    let event = parse_event(
        "MESSAGE_REACTION_ADD",
        json!({
            "user_id": "42", "channel_id": "7", "message_id": "1000",
            "guild_id": "9", "emoji": { "id": null, "name": "🔥" },
            "burst": false, "type": 0
        }),
    )
    .unwrap();
    let Event::ReactionAdd(add) = event else {
        panic!("expected ReactionAdd, got {event:?}")
    };
    assert_eq!(add.emoji.name.as_deref(), Some("🔥"));

    let event = parse_event(
        "MESSAGE_REACTION_REMOVE",
        json!({
            "user_id": "42", "channel_id": "7", "message_id": "1000",
            "emoji": { "id": null, "name": "🔥" }, "burst": false, "type": 0
        }),
    )
    .unwrap();
    assert!(matches!(event, Event::ReactionRemove(_)));
}

#[test]
fn parses_member_events() {
    let user = json!({ "id": "42", "username": "tester", "discriminator": "0" });

    let event = parse_event(
        "GUILD_MEMBER_ADD",
        json!({
            "guild_id": "9", "user": user, "roles": [],
            "deaf": false, "mute": false,
            "joined_at": "2015-04-26T06:26:56.936000+00:00"
        }),
    )
    .unwrap();
    let Event::MemberAdd(add) = event else {
        panic!("expected MemberAdd, got {event:?}")
    };
    assert_eq!(add.guild_id.get(), 9);
    assert_eq!(add.member.user.as_ref().unwrap().username, "tester");

    let event = parse_event(
        "GUILD_MEMBER_UPDATE",
        json!({ "guild_id": "9", "user": user, "roles": ["15"], "deaf": false, "mute": false }),
    )
    .unwrap();
    let Event::MemberUpdate(update) = event else {
        panic!("expected MemberUpdate, got {event:?}")
    };
    assert_eq!(update.member.roles.len(), 1);

    let event = parse_event(
        "GUILD_MEMBER_REMOVE",
        json!({ "guild_id": "9", "user": user }),
    )
    .unwrap();
    assert!(matches!(event, Event::MemberRemove(_)));
}

#[test]
fn parses_guild_events() {
    let guild_base = json!({
        "id": "9", "name": "test guild", "owner_id": "42",
        "afk_timeout": 300, "verification_level": 0,
        "default_message_notifications": 0, "explicit_content_filter": 0,
        "roles": [], "emojis": [], "features": [], "mfa_level": 0,
        "premium_tier": 0, "preferred_locale": "en-US", "nsfw_level": 0,
        "premium_progress_bar_enabled": false
    });

    let mut create = guild_base.clone();
    create["joined_at"] = json!("2015-04-26T06:26:56.936000+00:00");
    create["large"] = json!(false);
    create["member_count"] = json!(3);
    create["members"] = json!([]);
    create["channels"] = json!([{ "id": "7", "type": 0, "name": "general" }]);

    let event = parse_event("GUILD_CREATE", create).unwrap();
    let Event::GuildCreate(payload) = event else {
        panic!("expected GuildCreate, got {event:?}")
    };
    assert_eq!(payload.guild.name, "test guild");
    assert_eq!(payload.member_count, Some(3));
    assert_eq!(payload.channels.as_ref().unwrap().len(), 1);

    let event = parse_event("GUILD_UPDATE", guild_base).unwrap();
    let Event::GuildUpdate(guild) = event else {
        panic!("expected GuildUpdate, got {event:?}")
    };
    assert_eq!(guild.name, "test guild");

    let event = parse_event("GUILD_DELETE", json!({ "id": "9" })).unwrap();
    let Event::GuildDelete(guild) = event else {
        panic!("expected GuildDelete, got {event:?}")
    };
    // Absent `unavailable`: the bot left or was removed, not an outage.
    assert_eq!(guild.unavailable, None);
}

#[test]
fn parses_channel_and_role_events() {
    let channel = json!({ "id": "7", "type": 0, "name": "general" });
    for kind in ["CHANNEL_CREATE", "CHANNEL_UPDATE", "CHANNEL_DELETE"] {
        let event = parse_event(kind, channel.clone()).unwrap();
        assert!(
            matches!(
                event,
                Event::ChannelCreate(_) | Event::ChannelUpdate(_) | Event::ChannelDelete(_)
            ),
            "{kind} parsed into the wrong variant"
        );
    }

    let role_change = json!({
        "guild_id": "9",
        "role": {
            "id": "15", "name": "mods", "color": 0,
            "colors": { "primary_color": 0 }, "hoist": false, "position": 1,
            "permissions": "40", "managed": false, "mentionable": true
        }
    });
    let event = parse_event("GUILD_ROLE_CREATE", role_change.clone()).unwrap();
    let Event::RoleCreate(change) = event else {
        panic!("expected RoleCreate, got {event:?}")
    };
    assert_eq!(change.role.name, "mods");

    let event = parse_event("GUILD_ROLE_UPDATE", role_change).unwrap();
    assert!(matches!(event, Event::RoleUpdate(_)));

    let event = parse_event(
        "GUILD_ROLE_DELETE",
        json!({ "guild_id": "9", "role_id": "15" }),
    )
    .unwrap();
    let Event::RoleDelete(delete) = event else {
        panic!("expected RoleDelete, got {event:?}")
    };
    assert_eq!(delete.role_id.get(), 15);
}

#[test]
fn parses_interaction_create() {
    let event = parse_event(
        "INTERACTION_CREATE",
        json!({
            "id": "1064160133216616528",
            "application_id": "80351110224678912",
            "type": 2,
            "data": {
                "id": "1064160133216616529", "name": "blep", "type": 1,
                "options": [ { "name": "animal", "type": 3, "value": "cat" } ]
            },
            "guild_id": "9",
            "channel_id": "7",
            "member": { "user": { "id": "42", "username": "t", "discriminator": "0" }, "roles": [], "deaf": false, "mute": false },
            "token": "token", "version": 1
        }),
    )
    .unwrap();
    let Event::InteractionCreate(interaction) = event else {
        panic!("expected InteractionCreate, got {event:?}")
    };
    assert_eq!(
        interaction.r#type,
        axis_model::InteractionType::ApplicationCommand
    );
    let data = interaction.command_data().unwrap();
    assert_eq!(data.name, "blep");
    assert_eq!(data.options[0].value.as_ref().unwrap(), "cat");
}

#[test]
fn unknown_kinds_become_unknown_events() {
    let event = parse_event(
        "SOUNDBOARD_SOUNDS",
        json!({ "guild_id": "9", "sounds": [] }),
    )
    .expect("unknown kinds never fail");
    let Event::Unknown { kind, data } = event else {
        panic!("expected Unknown, got {event:?}")
    };
    assert_eq!(kind, "SOUNDBOARD_SOUNDS");
    assert_eq!(data["guild_id"], "9");

    // RESUMED is a known kind with no payload.
    let event = parse_event("RESUMED", json!(null)).unwrap();
    assert!(matches!(event, Event::Resumed));
}

#[test]
fn corrupted_known_payloads_are_errors() {
    // MESSAGE_CREATE without the required author.
    let error = parse_event("MESSAGE_CREATE", json!({ "id": "1000", "channel_id": "7" }))
        .expect_err("missing author must fail");
    assert!(matches!(error, axis_core::AxisError::Serialization(_)));
}

#[tokio::test]
async fn dispatcher_routes_typed_handlers() -> Result<()> {
    let calls = Arc::new(Mutex::new(Vec::<String>::new()));

    let mut dispatcher = Dispatcher::new();

    let message_calls = Arc::clone(&calls);
    dispatcher.on_message_create(move |message| {
        let calls = Arc::clone(&message_calls);
        async move {
            calls
                .lock()
                .unwrap()
                .push(format!("create:{}", message.content));
            Ok(())
        }
    });

    // A second handler on the same kind: runs after the first, in order.
    let second_calls = Arc::clone(&calls);
    dispatcher.on_message_create(move |message| {
        let calls = Arc::clone(&second_calls);
        async move {
            calls
                .lock()
                .unwrap()
                .push(format!("create-2:{}", message.id));
            Ok(())
        }
    });

    let delete_calls = Arc::clone(&calls);
    dispatcher.on_message_delete(move |delete| {
        let calls = Arc::clone(&delete_calls);
        async move {
            calls.lock().unwrap().push(format!("delete:{}", delete.id));
            Ok(())
        }
    });

    dispatcher
        .dispatch(parse_event(
            "MESSAGE_CREATE",
            json!({
                "id": "1000", "channel_id": "7",
                "author": { "id": "42", "username": "t", "discriminator": "0" },
                "content": "hello", "timestamp": "2022-09-22T14:00:00.436000+00:00",
                "tts": false, "mention_everyone": false, "mentions": [],
                "mention_roles": [], "attachments": [], "embeds": [], "pinned": false, "type": 0
            }),
        )?)
        .await
        .unwrap();
    dispatcher
        .dispatch(parse_event(
            "MESSAGE_DELETE",
            json!({ "id": "1000", "channel_id": "7" }),
        )?)
        .await
        .unwrap();
    // Unrelated kind: no typed handler runs.
    dispatcher
        .dispatch(parse_event(
            "GUILD_ROLE_DELETE",
            json!({ "guild_id": "9", "role_id": "15" }),
        )?)
        .await
        .unwrap();

    let calls = calls.lock().unwrap().clone();
    assert_eq!(
        calls,
        vec!["create:hello", "create-2:1000", "delete:1000"],
        "handlers run in registration order, only for their kind"
    );
    Ok(())
}

#[tokio::test]
async fn catch_all_and_unknown_handlers() -> Result<()> {
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));

    let mut dispatcher = Dispatcher::new();
    let any_seen = Arc::clone(&seen);
    dispatcher.on_any(move |event| {
        let seen = Arc::clone(&any_seen);
        async move {
            seen.lock()
                .unwrap()
                .push(format!("any:{:?}", std::mem::discriminant(&event)));
            Ok(())
        }
    });
    let unknown_seen = Arc::clone(&seen);
    dispatcher.on_unknown(move |kind, data| {
        let seen = Arc::clone(&unknown_seen);
        async move {
            seen.lock().unwrap().push(format!(
                "unknown:{kind}:{}",
                data["guild_id"].as_str().unwrap_or_default()
            ));
            Ok(())
        }
    });

    dispatcher
        .dispatch(parse_event("RESUMED", json!(null))?)
        .await
        .unwrap();
    dispatcher
        .dispatch(parse_event(
            "SOUNDBOARD_SOUNDS",
            json!({ "guild_id": "9", "sounds": [] }),
        )?)
        .await
        .unwrap();

    let seen = seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 3, "any runs for both, unknown only for Unknown");
    assert!(seen[0].starts_with("any:"), "any runs first");
    assert!(seen[1].starts_with("any:"));
    assert_eq!(seen[2], "unknown:SOUNDBOARD_SOUNDS:9");
    Ok(())
}

#[tokio::test]
async fn handler_errors_stop_later_handlers_and_propagate() -> Result<()> {
    let calls = Arc::new(AtomicUsize::new(0));

    let mut dispatcher = Dispatcher::new();
    let first = Arc::clone(&calls);
    dispatcher.on_ready(move |_| {
        let calls = Arc::clone(&first);
        async move {
            calls.fetch_add(1, Ordering::SeqCst);
            Err(axis_core::AxisError::invalid_data("handler boom"))
        }
    });
    let second = Arc::clone(&calls);
    dispatcher.on_ready(move |_| {
        let calls = Arc::clone(&second);
        async move {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    });

    let event = parse_event(
        "READY",
        json!({
            "v": 10,
            "user": { "id": "1", "username": "b", "discriminator": "0" },
            "guilds": [], "session_id": "s",
            "resume_gateway_url": "wss://gateway.discord.gg"
        }),
    )?;

    let error = dispatcher
        .dispatch(event)
        .await
        .expect_err("first handler fails");
    assert!(error.to_string().contains("handler boom"));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "later handlers are skipped"
    );
    Ok(())
}

#[tokio::test]
async fn run_loop_skips_parse_failures_and_survives_handler_errors() -> Result<()> {
    let good_messages = Arc::new(AtomicUsize::new(0));

    let mut dispatcher = Dispatcher::new();
    let counter = Arc::clone(&good_messages);
    dispatcher.on_message_create(move |message| {
        let counter = Arc::clone(&counter);
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            if message.content == "boom" {
                return Err(axis_core::AxisError::invalid_data("boom"));
            }
            Ok(())
        }
    });

    let valid_message = json!({
        "id": "1000", "channel_id": "7",
        "author": { "id": "42", "username": "t", "discriminator": "0" },
        "content": "hello", "timestamp": "2022-09-22T14:00:00.436000+00:00",
        "tts": false, "mention_everyone": false, "mentions": [],
        "mention_roles": [], "attachments": [], "embeds": [], "pinned": false, "type": 0
    });

    // Sequence: broken payload (parse error, skipped), good message whose
    // handler fails (logged, run continues), another good message.
    let shard_events = vec![
        Ok(ShardEvent::Dispatch {
            kind: "MESSAGE_CREATE".to_owned(),
            data: json!({ "id": "1000", "channel_id": "7" }), // no author
        }),
        Ok(ShardEvent::Dispatch {
            kind: "MESSAGE_CREATE".to_owned(),
            data: {
                let mut m = valid_message.clone();
                m["content"] = json!("boom");
                m
            },
        }),
        Ok(ShardEvent::Dispatch {
            kind: "MESSAGE_CREATE".to_owned(),
            data: valid_message,
        }),
    ];

    dispatcher
        .run(stream::iter(shard_events))
        .await
        .expect("run survives parse and handler errors");
    assert_eq!(
        good_messages.load(Ordering::SeqCst),
        2,
        "both valid messages reached handlers despite one failing"
    );
    Ok(())
}

#[tokio::test]
async fn run_loop_propagates_stream_errors() {
    let dispatcher = Dispatcher::new();
    let error = dispatcher
        .run(stream::iter(vec![Err(axis_core::AxisError::invalid_data(
            "gateway gave up",
        ))]))
        .await
        .expect_err("stream errors are fatal");
    assert!(error.to_string().contains("gateway gave up"));
}
