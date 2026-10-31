# Axis

An independent, general-purpose [Discord API](https://discord.com/developers/docs/intro) library for Rust.

Axis is **not** a Discord bot — it is the foundation bots are built on: a typed, async, modular library covering Discord's REST API, Gateway, data models, events, and an optional command framework. It is written from scratch against the official Discord API documentation and shares no code with Serenity, Twilight, or other libraries.

## Status

Axis is under active development and follows a phased plan (see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)).

| Area | Crate | Status |
|---|---|---|
| Foundation: IDs, timestamps, intents, permissions, errors, logging | [`axis-core`](crates/axis-core) | ✅ Phase 1 |
| Data models | [`axis-model`](crates/axis-model) | ✅ REST-surface models; expands in Phase 4 |
| Facade, Client, prelude | [`axis`](crates/axis) | ✅ Phase 1–5 |
| REST client, rate limits | [`axis-rest`](crates/axis-rest) | ✅ Phase 2 |
| Gateway (WebSocket) shards | [`axis-gateway`](crates/axis-gateway) | ✅ Phase 3 |
| Event system | [`axis-events`](crates/axis-events) | ✅ Phase 4 |
| Command framework | [`axis-commands`](crates/axis-commands) | ✅ Phase 5 |

> The `Client` API below is **implemented** — the original design target, now real: REST (Phase 2), gateway (Phase 3), events (Phase 4), and commands (Phase 5) compose exactly as first sketched.

```rust,ignore
use axis::prelude::*;

#[tokio::main]
async fn main() -> Result<(), AxisError> {
    let client = Client::builder()
        .token(Token::from_env("DISCORD_TOKEN")?)
        .intents(GatewayIntents::unprivileged() | GatewayIntents::MESSAGE_CONTENT)
        .build()
        .await?;

    client.on_message_create(|ctx, message| async move {
        if message.content == "!hello" {
            ctx.http
                .create_message(message.channel_id)
                .content("Hello from Axis!")
                .await?;
        }
        Ok(())
    });

    client.start().await?;
    Ok(())
}
```

## Installation

Axis is not published to crates.io yet. Once hosted in a git repository:

```toml
[dependencies]
axis = { git = "https://example.com/axis" }
```

## What Phase 4 provides — models and events

Raw gateway dispatches become typed events with a handler dispatcher:

```rust
use axis::prelude::*;

let mut dispatcher = Dispatcher::new();
dispatcher
    .on_message_create(|message| async move {
        if message.content == "!hello" {
            tracing::info!("hello from {}", message.author.username);
        }
        Ok(())
    })
    .on_reaction_add(|reaction| async move {
        tracing::info!(emoji = ?reaction.emoji.name, "reaction added");
        Ok(())
    })
    .on_any(|event| async move {
        tracing::trace!(event = ?std::mem::discriminant(&event), "event");
        drop(event);
        Ok(())
    });

// Feed it a shard's event stream (runs until the stream ends):
// dispatcher.run(shard_events).await?;
```

- **Typed event enum** — `Ready`, message lifecycle (`MessageCreate`, partial `MessageUpdate`, deletes, bulk deletes), reactions, member join/update/leave, guild create/update/delete, channel lifecycle, roles, and `InteractionCreate` — every payload verified against the official event documentation.
- **Unknown events never break the bot**: dispatch kinds Axis does not model yet surface as `Event::Unknown { kind, data }` (register `on_unknown` to observe them); parse failures on known kinds are logged and skipped; handler errors stop later handlers for that event but never the run loop.
- **`Dispatcher`** — typed per-event registration in registration order, a catch-all `on_any`, an `on_unknown` hook, and a `run` loop that consumes a shard's event stream (stream errors are the only fatal ones).
- **Model expansion** — `PartialMessage`, delete/bulk-delete and reaction payloads, flat member-event payloads (`MemberAdd`/`MemberUpdate`/`MemberRemove`), `GuildCreate` with embedded channels/members, `UnavailableGuild`, `Ready`, `Application`/`PartialApplication`, `Interaction` (with command data decoding groundwork for Phase 5), and the complete `AuditLogEntry` with all ~75 documented action types.

## What Phase 5 provides — commands and the Client

The layers compose into the client the README opened with:

```rust
use axis::prelude::*;

#[tokio::main]
async fn main() -> Result<(), AxisError> {
    let mut registry = CommandRegistry::new();
    registry.command("echo", "Repeat after you", |cmd| {
        cmd.option(
            CommandOption::string("text", "What to echo").required().finish(),
        )
        .handler(|invocation| async move {
            let text: String = invocation.arg("text")?;
            invocation.reply().content(text).await?;
            Ok(())
        });
    });

    let mut client = Client::builder(Token::from_env("DISCORD_TOKEN")?)
        .intents(GatewayIntents::unprivileged() | GatewayIntents::MESSAGE_CONTENT)
        .commands(registry)
        .build()
        .await?;

    client.on_message_create(|ctx, message| async move {
        if message.content == "!hello" {
            ctx.http
                .create_message(message.channel_id)
                .content("Hello from Axis!")
                .await?;
        }
        Ok(())
    });

    // Sync the command set once (global bulk overwrite):
    // registry.register_global(&client.http(), application_id).await?;

    client.start().await?;
    Ok(())
}
```

- **`Client`** — `build()` fetches the recommended gateway URL (`GET /gateway`), wires the shard + dispatcher + command registry; `start()` runs until the shard stops; `shutdown()` closes gracefully; `on_*` handlers receive a `Context` with the REST client.
- **Command framework** — typed options (string/integer/number/boolean/user/channel/role/mentionable/attachment) with choices, bounds, and lengths; subcommands and subcommand groups with their own handlers; `default_member_permissions`, `dm_permission`, and `nsfw` flags; declaration validation (required-before-optional, name/description rules, handler presence).
- **Registration** — bulk-overwrite (global or per-guild) through the rate-limit-aware REST client, plus command permissions (`GET`/`PUT`).
- **Invocation handling** — typed argument extraction (`invocation.arg::<T>()`), subcommand-path resolution, reply / defer / ephemeral responses and followups through the interaction callback + webhook endpoints.
- **Autocomplete** — per-option providers that inspect the focused input and answer with choices.
- **Token safety** — interaction tokens ride in URLs (as the API requires) but error routes are sanitized (`/interactions/900/[token]/callback`), so credentials never reach logs.

## What Phase 1 provides

Everything below is implemented and tested:

- **Typed snowflake IDs** — `Id<T>` with zero-sized marker types (`UserId`, `GuildId`, `ChannelId`, …), string-or-number deserialization, and snowflake timestamp decoding (`id.created_at()`).
- **`GatewayIntents`** — the complete documented intent set with the exact bit values from Discord's Gateway docs, including helpers for the three privileged intents.
- **`Permissions`** — the full documented permission bit table (`1 << 0` through `1 << 52`), serialized the way Discord does (decimal strings), with human-readable formatting for errors.
- **A complete error system** — `AxisError` with per-subsystem variants (gateway, HTTP, auth, rate limit, serialization, invalid data, permission, connection, internal), structured context (status + route + Discord error codes, retry durations, close codes), source chains, and a `is_transient()` classifier.
- **Gateway close-code semantics** — every documented close code (4000–4014) with correct reconnect/resume classification, ready for the gateway phase: close codes 4004 and 4010–4014 are non-reconnectable; 4007/4009 require a fresh session instead of a resume.
- **`Timestamp`** — RFC 3339 timestamps with serde support matching Discord's format.
- **`Secret`/`Token`** — credentials that cannot accidentally leak: redacted `Debug`, no `Display`, no serde, plus a `from_env` constructor.
- **`User` model** — the first data model, establishing the patterns (optional fields everywhere the API is optional, unknown-field tolerance, typed IDs, bitset flags) the remaining models follow.
- **Logging** — structured logging via `tracing`, with an optional default-subscriber helper (Axis never installs global state on its own).


## What Phase 2 provides — the REST layer

The REST client is implemented and tested against a mock HTTP server:

```rust
use axis::prelude::*;

#[tokio::main]
async fn main() -> Result<(), AxisError> {
    let http = Http::builder(Token::from_env("DISCORD_TOKEN")?).build()?;

    // Awaitable, typed request builders:
    let message = http
        .create_message(ChannelId::new(123))
        .content("Hello from Axis!")
        .embeds(vec![Embed {
            title: Some("Axis".into()),
            description: Some("An independent Discord library".into()),
            color: Some(0x5865F2),
            ..Embed::default()
        }])
        .reply(MessageId::new(456))
        .await?;

    http.create_message(message.channel_id)
        .content("Pong!")
        .await?;
    Ok(())
}
```

- **25+ endpoints** behind typed builders: messages (create/edit/delete/get/list/bulk-delete with replies and embeds), channels (get/create/edit/delete, permission overwrites), guilds, members (get/kick/ban/unban/edit — including timeouts), roles (create/edit/delete), reactions (add/remove/clear with URL-encoded emoji).
- **Rate limits handled properly**: bucket hashes learned from `X-RateLimit-*` headers and keyed by major parameter; exhausted buckets wait for their reset instead of hammering Discord; 429s are honored via the body's `retry_after` (with `Retry-After` header fallback for Cloudflare bodies) and retried a bounded number of times before surfacing `RateLimitError`.
- **Bounded, safe retries**: network errors and 5xx retry with exponential backoff — only for idempotent methods (`GET`/`PUT`/`DELETE`), so a failed message send can never be silently duplicated. 401 stops immediately per Discord's guidance.
- **Typed errors end to end**: `401 → AuthError`, `429 → RateLimitError` (after bounded retries), other statuses → `HttpError` with the Discord error code + message parsed from the body.
- **Expanded models**: `Message`, `Channel`, `Guild`, `Member`, `Role`, `Embed` (and sub-structures), `Attachment`, `Reaction`, `Emoji`, plus the documented `ChannelType`/`MessageType` enums and `MessageFlags`.

## What Phase 3 provides — the gateway

A `Shard` maintains one gateway connection on a background task and streams
raw dispatch events:

```rust
use axis::prelude::*;

#[tokio::main]
async fn main() -> Result<(), AxisError> {
    let shard = Shard::builder(Token::from_env("DISCORD_TOKEN")?, GatewayIntents::unprivileged())
        .build()?;

    let mut events = shard.start();
    while let Some(event) = events.next_event().await {
        match event? {
            ShardEvent::Dispatch { kind, data } => {
                if kind == "MESSAGE_CREATE" {
                    tracing::info!(?data, "new message");
                }
            }
            ShardEvent::Reconnecting { close_code } => {
                tracing::warn!(?close_code, "shard reconnecting");
            }
            _ => {}
        }
    }
    Ok(())
}
```

- **Protocol-complete per the docs**: Hello → jittered first heartbeat → Identify (token without `Bot ` prefix, intents, shard pair, connection properties) → heartbeats carrying the last sequence number → Heartbeat ACK tracking.
- **Zombie detection**: no heartbeat ack between beats → the connection is terminated with a session-preserving close code and resumed.
- **Reconnect vs resume, decided correctly** (the core requirement): op 7 Reconnect, resumable op 9 Invalid Session, transport loss, and resumable close codes → **Resume** (to the `resume_gateway_url` from READY, with `session_id` + `seq`). Invalid Session with `d: false`, close codes 1000/1001, and 4007/4009 → fresh **Identify**. Fatal codes (4004, 4010–4014) end the stream with a typed error instead of retrying forever.
- **Sequence tracking and session management** live in one place (`Session`), so a reconnect never blindly re-identifies.
- **Graceful shutdown**: `Shard::shutdown()` (or dropping the handle) closes with code 1000 and ends the event stream; dropping the stream's consumer also winds the shard down.
- **Sharding-ready**: one `Shard` is one connection; `ShardBuilder::shard(id, total)` puts the shard pair in Identify.
- **Tested over real sockets**: 13 pipeline tests run against a scripted local WebSocket server — identify shape, heartbeat/ack flow, op-7 resume, invalid-session both ways, close-code-driven resume vs identify, zombie teardown, transport drops, fatal close codes, shutdown, and reconnect-attempt caps.

## Design principles

- **Lightweight and modular.** The workspace is split by responsibility; the core crate has no async runtime, HTTP, or WebSocket dependencies. Heavy dependencies (reqwest, tokio) live in the crates that actually need them.
- **Spec-faithful.** Bit values, field names, serialization shapes, and recovery semantics follow the official Discord API documentation, verified field-by-field during implementation.
- **Robust against change.** Unknown JSON fields are ignored; unknown bit flags are preserved or dropped where harmless; optionality mirrors the API's own optionality.
- **No fake APIs.** Crates advertise only what they implement. Planned functionality is documented as planned.
- **Security-conscious.** Tokens are never hardcoded or logged; configuration via environment; no support for automating normal user accounts.

## Security notes

- Provide the token through the environment (`DISCORD_TOKEN`) or your own secret store — never in source code.
- Axis never logs credentials; the `Secret`/`Token` wrapper makes accidental leakage hard.
- Privileged intents (`GUILD_PRESENCES`, `GUILD_MEMBERS`, `MESSAGE_CONTENT`) must be enabled in the Discord Developer Portal; the library surfaces Discord's close codes 4013/4014 instead of hiding them.
- Axis is for bot/application accounts only. Automating normal user accounts violates Discord's Terms of Service and is not supported.

## Documentation

- [Getting started](docs/GETTING_STARTED.md) — install, authenticate,
  quick start, then gateway, REST, events, commands, and error handling.
- [Architecture](docs/ARCHITECTURE.md) — crate layout, design decisions,
  roadmap.
- [Examples](examples/README.md) — runnable bots: `basic_bot`,
  `event_handler`, `slash_commands`.

## Development

```sh
cargo check --workspace --all-targets   # compile everything (incl. examples)
cargo test --workspace                  # unit + integration + soak tests
cargo fmt --all                         # formatting
cargo clippy --workspace --all-targets  # lints
```

Rust 1.85+ (edition 2024). CI runs all four on every push; the test suite
is fully offline (mocked REST and gateway servers), so no Discord token is
required to develop or contribute.

## License

Dual-licensed under either of:

- Apache License, Version 2.0 — [LICENSE-APACHE](LICENSE-APACHE)
- MIT License — [LICENSE-MIT](LICENSE-MIT)

at your option.
