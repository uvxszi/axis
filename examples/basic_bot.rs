//! The minimal Axis bot: connect, listen for `!hello`, and reply.
//!
//! Run with `DISCORD_TOKEN=<bot token> cargo run -p axis-examples --bin basic_bot`.
//!
//! Required setup:
//! - a bot application (Discord Developer Portal),
//! - the `MESSAGE_CONTENT` privileged intent enabled in the portal, since
//!   this bot reads message text.

use axis::prelude::*;

#[tokio::main]
async fn main() -> Result<(), AxisError> {
    // Optional: install a default `tracing` subscriber so Axis's internal
    // logs (reconnects, rate limits) are visible. RUST_LOG overrides the
    // filter, e.g. `RUST_LOG=axis=debug`.
    axis::log::init()?;

    let token = Token::from_env("DISCORD_TOKEN")
        .map_err(|_| AxisError::invalid_data("set DISCORD_TOKEN to your bot token"))?;

    let mut client = Client::builder(token)
        .intents(GatewayIntents::unprivileged() | GatewayIntents::MESSAGE_CONTENT)
        .build()
        .await?;

    client.on_message_create(|ctx, message| async move {
        // Never respond to bots (including ourselves).
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

    println!("basic_bot running; press Ctrl+C to stop");
    tokio::select! {
        result = client.start() => {
            // The shard gave up (fatal close code) or was capped.
            result?;
        }
        _ = tokio::signal::ctrl_c() => {
            println!("shutting down");
            // Close the gateway gracefully (code 1000). The shard task
            // winds down in the background; give it a moment before the
            // process exits.
            client.shutdown();
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    Ok(())
}
