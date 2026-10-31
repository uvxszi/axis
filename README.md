# Axis

An independent, general-purpose [Discord API](https://discord.com/developers/docs/intro) library for Rust.

Axis is **not** a Discord bot — it is the foundation bots are built on: a typed, async, modular library covering Discord's REST API, Gateway, data models, events, and an optional command framework. It is written from scratch against the official Discord API documentation and shares no code with Serenity, Twilight, or other libraries.

## Status

Axis is under active development and follows a phased plan (see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)).

| Area | Crate | Status |
|---|---|---|
| Foundation: IDs, timestamps, intents, permissions, errors, logging | [`axis-core`](crates/axis-core) | ✅ Phase 1 |
| Data models | [`axis-model`](crates/axis-model) | 🚧 Phase 1 started (`User`); expands in Phase 4 |
| Facade + prelude | [`axis`](crates/axis) | ✅ Phase 1 |
| REST client, rate limits | [`axis-rest`](crates/axis-rest) | ⏳ Phase 2 |
| Gateway (WebSocket) client | [`axis-gateway`](crates/axis-gateway) | ⏳ Phase 3 |
| Event system | [`axis-events`](crates/axis-events) | ⏳ Phase 4 |
| Command framework | [`axis-commands`](crates/axis-commands) | ⏳ Phase 5 |

> The `Client` API shown below is the **design target** for Phases 2–5, not working code yet. Phase 1 ships the workspace, core types, and the error system — everything is tested and compiling.

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

Until the REST and Gateway phases land, depending on `axis` gives you the foundation crates only.

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
