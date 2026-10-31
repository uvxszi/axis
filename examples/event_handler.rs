//! A tour of Axis's event handlers: messages, reactions, members, guilds,
//! channels, and roles, with lifecycle visibility.
//!
//! Run with `DISCORD_TOKEN=<bot token> cargo run -p axis-examples --bin event_handler`.
//!
//! Member and guild events require the `GUILD_MEMBERS` privileged intent
//! in the Developer Portal; without it Discord simply stops sending them.

use axis::prelude::*;

#[tokio::main]
async fn main() -> Result<(), AxisError> {
    axis::log::init()?;

    let token = Token::from_env("DISCORD_TOKEN")
        .map_err(|_| AxisError::invalid_data("set DISCORD_TOKEN to your bot token"))?;

    let mut client = Client::builder(token)
        .intents(
            GatewayIntents::unprivileged()
                | GatewayIntents::GUILD_MEMBERS
                | GatewayIntents::MESSAGE_CONTENT,
        )
        .build()
        .await?;

    client
        .on_ready(|_ctx, ready| async move {
            tracing::info!(
                user = %ready.user.username,
                guilds = ready.guilds.len(),
                "session established"
            );
            Ok(())
        })
        .on_message_create(|_ctx, message| async move {
            tracing::info!(author = %message.author.username, content = %message.content, "message");
            Ok(())
        })
        .on_message_delete(|_ctx, delete| async move {
            tracing::info!(message = %delete.id, "message deleted");
            Ok(())
        })
        .on_reaction_add(|_ctx, reaction| async move {
            tracing::info!(
                message = %reaction.message_id,
                emoji = ?reaction.emoji.name,
                "reaction added"
            );
            Ok(())
        })
        .on_member_add(|_ctx, add| async move {
            match &add.member.user {
                Some(user) => {
                    tracing::info!(guild = %add.guild_id, user = %user.id, "member joined");
                }
                None => tracing::info!(guild = %add.guild_id, "member joined"),
            }
            Ok(())
        })
        .on_member_remove(|_ctx, remove| async move {
            tracing::info!(guild = %remove.guild_id, user = %remove.user.id, "member left");
            Ok(())
        })
        .on_guild_create(|_ctx, create| async move {
            tracing::info!(
                guild = %create.guild.name,
                members = ?create.member_count,
                "guild available"
            );
            Ok(())
        })
        .on_channel_create(|_ctx, channel| async move {
            tracing::info!(channel = ?channel.name, kind = ?channel.r#type, "channel created");
            Ok(())
        })
        .on_role_create(|_ctx, change| async move {
            tracing::info!(role = %change.role.name, "role created");
            Ok(())
        })
        .on_any(|_ctx, event| async move {
            // Catch-all: every event passes through here first.
            let _ = std::mem::discriminant(&event);
            Ok(())
        })
        .on_unknown(|_ctx, kind, _data| async move {
            // Event kinds this Axis version does not model yet.
            tracing::debug!(kind, "unmodeled event");
            Ok(())
        });

    println!("event_handler running; press Ctrl+C to stop");
    tokio::select! {
        result = client.start() => {
            result?;
        }
        _ = tokio::signal::ctrl_c() => {
            client.shutdown();
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    Ok(())
}
