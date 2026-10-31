//! Cross-crate integration tests: these exercise Axis the way an
//! application does — through the `axis` facade and its prelude — rather
//! than through internal crate paths.

use axis::prelude::*;

/// Full user payload in the shape Discord sends it.
const FULL_USER: &str = r#"{
    "id": "80351110224678912",
    "username": "Nelly",
    "discriminator": "0",
    "global_name": "Nelly",
    "avatar": "3370dadd186e2fcd8e5d22feb5674c88",
    "flags": 64,
    "premium_type": 2,
    "public_flags": 131072
}"#;

#[test]
fn prelude_types_are_reachable_and_correct() {
    // Typed IDs: construction, raw access, Display.
    let user_id = UserId::new(80_351_110_224_678_912);
    assert_eq!(user_id.get(), 80_351_110_224_678_912);
    assert_eq!(user_id.to_string(), "80351110224678912");

    // Generic Id with an explicit marker.
    let generic: Id<marker::GuildMarker> = Id::new(42);
    let aliased: GuildId = Id::new(42);
    assert_eq!(generic.get(), aliased.get());

    // Snowflake timestamp decoding (this example user was created in 2015).
    let created = user_id.created_at();
    assert_eq!(created.as_datetime().year(), 2015);
}

#[test]
fn intents_and_permissions_flow_through_facade() {
    let intents = GatewayIntents::unprivileged();
    assert!(!intents.is_privileged());
    assert!(intents.contains(GatewayIntents::GUILDS));
    assert!(!intents.contains(GatewayIntents::MESSAGE_CONTENT));

    let perms = Permissions::SEND_MESSAGES | Permissions::EMBED_LINKS;
    assert_eq!(
        perms.names().collect::<Vec<_>>(),
        vec!["SEND_MESSAGES", "EMBED_LINKS"]
    );
    assert!(!perms.is_administrator());
}

#[test]
fn user_model_deserializes_through_facade() {
    let user: User = serde_json::from_str(FULL_USER).unwrap();

    assert_eq!(user.id, UserId::new(80_351_110_224_678_912));
    assert_eq!(user.username, "Nelly");
    assert_eq!(user.mention(), "<@80351110224678912>");
    assert!(!user.is_bot());
    assert_eq!(user.premium_type, Some(PremiumType::Nitro));
    assert_eq!(user.public_flags, Some(UserFlags::VERIFIED_DEVELOPER));
}

#[test]
fn error_system_flows_through_facade() {
    // Constructing and matching through the public prelude types.
    let error = AxisError::from(AuthError::Invalid);
    assert!(
        matches!(error, AxisError::Auth(AuthError::Invalid)),
        "unexpected error shape: {error:?}"
    );

    // Transient classification is part of the public contract.
    assert!(
        AxisError::invalid_data("x")
            .to_string()
            .starts_with("invalid data:")
    );
    assert!(!AxisError::invalid_data("x").is_transient());
}

#[test]
fn tokens_are_redacted() {
    let token = Token::new("Bot real-token-value".to_owned());
    let rendered = format!("{token:?}");
    assert_eq!(rendered, "Secret([REDACTED])");
    assert!(!rendered.contains("real-token-value"));
    // The prelude also gives access when the library needs it.
    assert_eq!(token.as_str(), "Bot real-token-value");
}

#[test]
fn timestamps_roundtrip_through_facade() {
    let ts: Timestamp = "2015-04-26T06:26:56.936000+00:00".parse().unwrap();
    let json = serde_json::to_string(&ts).unwrap();
    let parsed: Timestamp = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, ts);
    assert_eq!(Timestamp::UNIX_EPOCH.as_unix_millis(), 0);
}

#[test]
fn rest_client_builds_through_facade() {
    // The Phase 2 REST surface: builder never contacts Discord on build.
    let http = Http::builder(Token::new("Bot abc.def".to_owned()))
        .build()
        .unwrap();
    // Builders are created without sending anything.
    let _builder = http
        .create_message(ChannelId::new(1))
        .content("not sent yet");
}

#[tokio::test]
async fn rest_validation_runs_before_network() {
    // Pointed at a port where nothing listens: local validation must fail
    // before any connection is attempted.
    let http = Http::builder(Token::new("x".to_owned()))
        .api_base("http://127.0.0.1:9")
        .build()
        .unwrap();
    let error = http
        .create_message(ChannelId::new(1))
        .await
        .expect_err("empty message must be rejected locally");
    assert!(error.to_string().contains("content or at least one embed"));
}

#[tokio::test]
async fn events_parse_and_dispatch_through_facade() {
    // The Phase 4 flow: a raw gateway dispatch parses into a typed Event
    // and reaches the registered handler.
    let received = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));

    let mut dispatcher = Dispatcher::new();
    let sink = std::sync::Arc::clone(&received);
    dispatcher.on_message_create(move |message| {
        let sink = std::sync::Arc::clone(&sink);
        async move {
            sink.lock()
                .unwrap()
                .push(format!("{}:{}", message.author.username, message.content));
            Ok(())
        }
    });

    let event = Event::parse(
        "MESSAGE_CREATE",
        serde_json::json!({
            "id": "1000", "channel_id": "7",
            "author": { "id": "42", "username": "Nelly", "discriminator": "0" },
            "content": "hello", "timestamp": "2022-09-22T14:00:00.436000+00:00",
            "tts": false, "mention_everyone": false, "mentions": [],
            "mention_roles": [], "attachments": [], "embeds": [], "pinned": false, "type": 0
        }),
    )
    .unwrap();

    dispatcher.dispatch(event).await.unwrap();
    assert_eq!(*received.lock().unwrap(), vec!["Nelly:hello".to_owned()]);
}
