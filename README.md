# Axis

An independent, general-purpose [Discord API](https://discord.com/developers/docs/intro) library for Rust.

Axis is **not** a Discord bot — it is the foundation bots are built on: a typed, async, modular library covering Discord's REST API, Gateway, data models, events, and an optional command framework. It is written from scratch against the official Discord API documentation and shares no code with Serenity, Twilight, or other libraries.

## Status

Axis is under active development and follows a phased plan (see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)).

| Area | Crate | Status |
|---|---|---|
| Foundation: IDs, timestamps, intents, permissions, errors, logging | [`axis-core`](crates/axis-core) | ✅ Phase 1 |
| Data models | [`axis-model`](crates/axis-model) | ✅ REST-surface models (Phase 2); expands in Phase 4 |
| Facade + prelude | [`axis`](crates/axis) | ✅ Phase 1–2 |
| REST client, rate limits | [`axis-rest`](crates/axis-rest) | ✅ Phase 2 |
| Gateway (WebSocket) client | [`axis-gateway`](crates/axis-gateway) | ⏳ Phase 3 |
| Event system | [`axis-events`](crates/axis-events) | ⏳ Phase 4 |
| Command framework | [`axis-commands`](crates/axis-commands) | ⏳ Phase 5 |

> The `Client` API shown below is the **design target** for Phases 3–5 (it needs the gateway). The REST layer underneath it — `Http` — **is implemented and tested** (Phase 2).

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

## Development

```sh
cargo check --workspace --all-targets   # compile everything
cargo test --workspace                  # unit + integration tests
cargo fmt --all                         # formatting
cargo clippy --workspace --all-targets  # lints
```

Rust 1.85+ (edition 2024).

## License

Dual-licensed under either of:

- Apache License, Version 2.0 — [LICENSE-APACHE](LICENSE-APACHE)
- MIT License — [LICENSE-MIT](LICENSE-MIT)

at your option.
