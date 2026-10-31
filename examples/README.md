# Examples

Every example here is a **runnable, compilable program** built on the real
Axis API — the same `Client`, event, and command surfaces documented in
[`docs/GETTING_STARTED.md`](../docs/GETTING_STARTED.md).

| Example | Demonstrates | Requires |
|---|---|---|
| [`basic_bot.rs`](basic_bot.rs) | Client setup, token from the environment, intents, a `!hello` handler, graceful Ctrl+C shutdown | `MESSAGE_CONTENT` intent enabled in the portal |
| [`event_handler.rs`](event_handler.rs) | The handler tour: ready, messages, reactions, members, guilds, channels, roles, plus `on_any`/`on_unknown` lifecycle visibility | `GUILD_MEMBERS` + `MESSAGE_CONTENT` intents |
| [`slash_commands.rs`](slash_commands.rs) | Command declaration (options, choices, subcommands, autocomplete), `validate()`, global bulk-overwrite registration, invocation + defer/followup flows | `DISCORD_APPLICATION_ID` env var |

## Running

```sh
export DISCORD_TOKEN="<your bot token>"
export DISCORD_APPLICATION_ID="<numeric application id>"   # slash_commands only

cargo run -p axis-examples --bin basic_bot
cargo run -p axis-examples --bin event_handler
cargo run -p axis-examples --bin slash_commands
```

`RUST_LOG=axis=debug` turns up the library's internal logging (reconnects,
rate-limit waits); the examples install a default `tracing` subscriber for
you.

## Notes

- The examples are wired as binaries of the `axis-examples` workspace
  member, so `cargo check --workspace` compiles them in CI — an example
  that rots fails the build like any other code.
- The bots print guidance and exit with a clear error when the required
  environment variables are missing; no tokens are ever hardcoded.
- Ctrl+C triggers a graceful shutdown: the gateway connection is closed
  with code 1000.
