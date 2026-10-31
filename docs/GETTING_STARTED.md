# Getting Started with Axis

This guide walks from zero to a running bot, then covers each subsystem in
the order you are likely to meet it: authentication, events, REST calls,
slash commands, and error handling.

Axis is an independent, general-purpose Discord API library for Rust. It
is split into focused crates behind one facade — depending on `axis` is
enough for almost everything.

> **Status**: the REST client, gateway client, event system, command
> framework, and the `Client` that ties them together are implemented and
> tested. Axis is not yet published to crates.io; use a git dependency.

---

## 1. Installation

```toml
[dependencies]
axis = { git = "https://example.com/axis" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

Axis requires Rust 1.85+ (edition 2024). Tokio is the async runtime; add
the `macros` feature so `#[tokio::main]` works.

## 2. A bot token, safely

Create an application in the [Discord Developer
Portal](https://discord.com/developers/applications), add a bot, and copy
the token. Axis never wants that token in source code:

```rust
use axis::prelude::*;

// Reads the environment variable; empty/whitespace values are rejected.
let token = Token::from_env("DISCORD_TOKEN")?;
```

`Token` is a redacting wrapper: `Debug` prints `Secret([REDACTED])`,
there is no `Display`, and there are no serde impls — credentials cannot
accidentally reach logs, panics, or serialized output. Access is always
explicit (`token.as_str()`), and Axis itself never logs it.

Never automate a normal user account — bot tokens only. That is both
Discord's rule and Axis's assumption.

## 3. Quick start

```rust
use axis::prelude::*;

#[tokio::main]
async fn main() -> Result<(), AxisError> {
    // Optional: install a default tracing subscriber (RUST_LOG-aware).
    axis::log::init()?;

    let token = Token::from_env("DISCORD_TOKEN")?;

    let mut client = Client::builder(token)
        .intents(GatewayIntents::unprivileged() | GatewayIntents::MESSAGE_CONTENT)
        .build()
        .await?;

    client.on_message_create(|ctx, message| async move {
        if message.author.bot.unwrap_or(false) {
            return Ok(());
        }
        if message.content == "!hello" {
            ctx.http
                .create_message(message.channel_id)
                .content(format!("Hello, <@{}>!", message.author.id))
                .await?;
        }
        Ok(())
    });

    println!("running; press Ctrl+C to stop");
    tokio::select! {
        result = client.start() => result?,
        _ = tokio::signal::ctrl_c() => {
            client.shutdown(); // graceful gateway close (code 1000)
        }
    }
    Ok(())
}
```

What happens under the hood:

1. `build()` creates the REST client (rate-limit aware) and fetches the
   recommended gateway URL via `GET /gateway`.
2. `start()` opens the gateway connection, sends `Identify` with your
   intents, heartbeats, and streams typed events to your handlers.
3. The shard survives network blips: it resumes sessions when it can and
   re-identifies when it must (see [Gateway](#5-gateway-usage)).

Runnable versions of everything below live in
[`examples/`](examples/README.md): `basic_bot`, `event_handler`, and
`slash_commands`.

## 4. Intents

Intents declare which event groups you receive; they are bitsets:

```rust
let intents = GatewayIntents::unprivileged() | GatewayIntents::MESSAGE_CONTENT;
let intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES;
```

Three are **privileged** and must also be toggled in the Developer Portal:
`GUILD_PRESENCES`, `GUILD_MEMBERS`, `MESSAGE_CONTENT`. If the portal
setting is missing, Discord closes the connection with close code 4014 —
Axis surfaces that as a fatal error instead of retrying forever, because
retrying cannot fix a configuration problem.

`GatewayIntents::unprivileged()` is the builder default; everything else
is opt-in.

## 5. Gateway usage

`Client` runs one shard — one gateway connection. Sharded bots set the
pair explicitly:

```rust
let mut client = Client::builder(token)
    .intents(intents)
    .shard(0, 2) // shard 0 of 2
    .build()
    .await?;
```

Reconnection semantics follow the official protocol exactly:

- **Resume** when possible: after server-requested reconnects, resumable
  closes, or transport loss — using `session_id` + last sequence +
  `resume_gateway_url` from READY.
- **Fresh identify** after closes 4007/4009 and 1000/1001 (the session is
  gone) — back at the original gateway URL.
- **Hard stop** on 4004 (bad token) and 4010–4014 (bad shard setup or
  intents): the stream ends with a typed error because retrying cannot
  help.
- **Zombie detection**: a heartbeat without its acknowledgement ends the
  connection and resumes, per the protocol's own guidance.

You can observe all of it through the lifecycle handlers:

```rust
client.on_any(|_ctx, event| async move {
    match &event {
        Event::Resumed => tracing::info!("session resumed"),
        _ => {}
    }
    Ok(())
});
```

## 6. REST usage

Every REST operation is a method on [`Http`](axis::rest::Http) returning
an awaitable, typed builder:

```rust
let http = client.http();

// Send a message with an embed.
http.create_message(channel_id)
    .content("Build status:")
    .embeds(vec![Embed {
        title: Some("main".into()),
        description: Some("all green".into()),
        color: Some(0x57F287),
        ..Embed::default()
    }])
    .await?;

// Edit and delete (deletes return ()).
http.edit_message(channel_id, message_id).content("fixed").await?;
http.delete_message(channel_id, message_id).await?;

// Moderation, channels, roles, reactions:
http.ban_member(guild_id, user_id).delete_message_seconds(86_400).await?;
http.create_guild_channel(guild_id, "new-text").kind(ChannelType::GuildText).await?;
http.add_reaction(channel_id, message_id, emoji).await?;
```

**Rate limits are handled for you**: bucket limits are learned from
response headers and tracked per (bucket, major parameter); exhausted
buckets wait for their reset; 429s honor `retry_after` and are retried a
bounded number of times; 5xx retries are restricted to idempotent methods
(`GET`/`PUT`/`DELETE`) so a failed `POST` can never be silently
duplicated.

## 7. Event handling

Handlers are async closures registered per event kind; they run in
registration order for each event:

```rust
client.on_message_create(|ctx, message| async move { /* ... */ Ok(()) });
client.on_reaction_add(|_ctx, reaction| async move { /* ... */ Ok(()) });
client.on_member_add(|_ctx, add| async move { /* ... */ Ok(()) });
client.on_guild_create(|_ctx, create| async move { /* ... */ Ok(()) });
client.on_any(|_ctx, event| async move { /* catch-all, runs first */ Ok(()) });
```

The full set: `on_ready`, `on_message_{create,update,delete,delete_bulk}`,
`on_reaction_{add,remove}`, `on_member_{add,update,remove}`,
`on_guild_{create,update,delete}`, `on_channel_{create,update,delete}`,
`on_role_{create,update,delete}`, `on_interaction_create`, plus `on_any`
and `on_unknown`.

Payloads are strongly typed (`Message`, `Member`, `Guild`, ...). Fields
the API marks optional are `Option<T>`; unknown JSON fields are ignored,
and event kinds Axis does not model yet surface as
`Event::Unknown { kind, data }` instead of failing — a running bot
survives Discord adding events.

Handler errors stop later handlers *for that event* but never the bot;
the run loop logs them and continues. Only a fatal gateway error ends
`client.start()`.

## 8. Command framework

Declare commands on a registry, validate, sync with Discord, and serve
invocations:

```rust
use axis::commands::CommandOption;

let mut registry = CommandRegistry::new();

registry.command("echo", "Repeat after you", |cmd| {
    cmd.option(
        CommandOption::string("text", "What to echo").required().finish(),
    )
    .option(CommandOption::boolean("ephemeral", "Just for you").finish())
    .handler(|invocation| async move {
        let text: String = invocation.arg("text")?;
        let ephemeral = invocation.opt::<bool>("ephemeral")?.unwrap_or(false);
        invocation.reply().content(text).ephemeral(ephemeral).await?;
        Ok(())
    });
});

registry.command("config", "Configure the bot", |cmd| {
    cmd.subcommand("set", "Set a value", |sub| {
        sub.option(
            CommandOption::string("key", "Which value").required().finish(),
        )
        .option(
            CommandOption::string("value", "The new value").required().finish(),
        )
        .handler(|invocation| async move {
            invocation.defer(false).await?;              // loading state
            invocation.followup().content("done").await?; // ~15 min of token
            Ok(())
        });
    });
});

registry.validate()?; // declaration mistakes fail before Discord ever sees them
```

Attach it to the client and sync once at startup:

```rust
let mut client = Client::builder(token)
    .intents(GatewayIntents::unprivileged())
    .commands(registry)
    .build()
    .await?;

client.http()
    .bulk_overwrite_global_commands(application_id, client.commands().unwrap().to_fields())
    .await?;
```

Interactions are dispatched automatically: the registry resolves the
subcommand path, extracts typed arguments, and invokes the handler. Long
work should `defer` first — an initial response is due within 3 seconds.

**Autocomplete**: attach a provider to any string/integer/number option;
it receives the invocation (with the focused input) and returns up to 25
choices. Axis answers the type-8 callback for you — and answers *empty*
if no provider is registered, so the interaction never hangs.

**Permissions**: `cmd.default_member_permissions(Permissions::KICK_MEMBERS)`
sets the server-side requirement; `dm_permission(false)` keeps commands
out of DMs; `nsfw(true)` age-restricts. Per-guild overrides are available
through the permission endpoints.

## 9. Error handling

Everything funnels through one type:

```rust
match result {
    Err(AxisError::Auth(AuthError::Invalid)) => {
        // 401: stop and fix the token — never hammer the API.
    }
    Err(AxisError::RateLimit(e)) => {
        // Bounded retries exhausted: e.retry_after, e.global.
    }
    Err(AxisError::Http(e)) => {
        // Discord said no: e.status, e.route, e.code (e.g. 50001), e.message.
    }
    Err(AxisError::Gateway(e)) => {
        // Connection/protocol failures; closes carry GatewayCloseCode with
        // is_reconnectable()/should_resume() classification.
    }
    Err(other) => { /* invalid data, serialization, internal, ... */ }
    Ok(value) => { /* ... */ }
}
```

Design properties:

- **Errors are data**: status + route + Discord error code, retry
  durations, close codes — with `source()` chains preserved.
- **`is_transient()`** classifies whether retrying plausibly helps.
- **Credentials never appear in errors**: interaction tokens are
  sanitized out of route strings.
- Internal errors are bugs in Axis and say so.

## 10. Where to go next

- [`examples/`](examples/README.md) — compilable, runnable bots.
- [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) — crate layout, dependency
  graph, and every design decision with its reasoning.
- Crate docs: `cargo doc --open` covers the full API surface.
