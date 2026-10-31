//! Slash commands: declare a registry, sync it with Discord, and serve
//! invocations — including subcommands, choices, and autocomplete.
//!
//! Run with
//! `DISCORD_TOKEN=<bot token> DISCORD_APPLICATION_ID=<app id> cargo run -p axis-examples --bin slash_commands`.
//!
//! The application ID is the numeric ID of your application (Developer
//! Portal → General Information). Registration uses a global bulk
//! overwrite; global commands can take up to an hour to appear, while
//! guild-scoped ones (`register_guild`) are instant.

use axis::prelude::*;

#[tokio::main]
async fn main() -> Result<(), AxisError> {
    axis::log::init()?;

    let token = Token::from_env("DISCORD_TOKEN")
        .map_err(|_| AxisError::invalid_data("set DISCORD_TOKEN to your bot token"))?;
    let application_id: u64 = std::env::var("DISCORD_APPLICATION_ID")
        .map_err(|_| AxisError::invalid_data("set DISCORD_APPLICATION_ID to your application ID"))?
        .parse()
        .map_err(|_| AxisError::invalid_data("DISCORD_APPLICATION_ID must be numeric"))?;

    let mut registry = CommandRegistry::new();

    registry.command("echo", "Repeat after you", |cmd| {
        cmd.option(
            CommandOption::string("text", "What to echo")
                .required()
                .max_length(200)
                .finish(),
        )
        .option(CommandOption::boolean("ephemeral", "Only you see the reply").finish())
        .handler(|invocation| async move {
            let text: String = invocation.arg("text")?;
            let ephemeral = invocation.opt::<bool>("ephemeral")?.unwrap_or(false);
            invocation
                .reply()
                .content(text)
                .ephemeral(ephemeral)
                .await?;
            Ok(())
        });
    });

    registry.command("config", "Configure the bot", |cmd| {
        cmd.subcommand("set", "Set a value", |sub| {
            sub.option(
                CommandOption::string("key", "Which value")
                    .required()
                    .choice("prefix", "prefix")
                    .choice("locale", "locale")
                    .finish(),
            )
            .option(
                CommandOption::string("value", "The new value")
                    .required()
                    .finish(),
            )
            .handler(|invocation| async move {
                let key: String = invocation.arg("key")?;
                let value: String = invocation.arg("value")?;
                // Longer work should defer first; the token allows
                // followups for ~15 minutes after a defer.
                invocation.defer(ephemeral_from(&invocation)).await?;
                invocation
                    .followup()
                    .content(format!("set {key} to {value}"))
                    .await?;
                Ok(())
            });
        })
        .subcommand("get", "Read a value", |sub| {
            sub.option(
                CommandOption::string("key", "Which value")
                    .required()
                    .choice("prefix", "prefix")
                    .choice("locale", "locale")
                    .finish(),
            )
            .handler(|invocation| async move {
                let key: String = invocation.arg("key")?;
                invocation
                    .reply()
                    .content(format!("{key} is currently unset"))
                    .ephemeral(true)
                    .await?;
                Ok(())
            });
        });
    });

    registry.command("pet", "Pet a virtual animal", |cmd| {
        cmd.option(
            CommandOption::string("animal", "Which animal")
                .autocomplete(|invocation| async move {
                    let typed = invocation.opt::<String>("animal")?.unwrap_or_default();
                    let animals = ["dog", "cat", "axolotl", "capybara"];
                    Ok(animals
                        .iter()
                        .filter(|name| name.starts_with(&typed.to_lowercase()))
                        .map(|name| axis::model::ApplicationCommandOptionChoice {
                            name: name.to_string(),
                            name_localizations: None,
                            value: serde_json::Value::String(name.to_string()),
                        })
                        .collect())
                })
                .finish(),
        )
        .handler(|invocation| async move {
            let animal: String = invocation.arg("animal")?;
            invocation
                .reply()
                .content(format!("*pets the {animal}*"))
                .await?;
            Ok(())
        });
    });

    // Catch declaration mistakes at startup, before Discord ever sees them.
    registry.validate()?;

    let client = Client::builder(token)
        .intents(GatewayIntents::unprivileged())
        .commands(registry)
        .build()
        .await?;

    // Sync the command set (bulk overwrite). Requires no extra scopes for
    // a bot token.
    client
        .http()
        .bulk_overwrite_global_commands(
            ApplicationId::new(application_id),
            client.commands().expect("registry attached").to_fields(),
        )
        .await?;
    tracing::info!("command set synced");

    println!("slash_commands running; press Ctrl+C to stop");
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

/// The visibility choice for deferred config updates.
fn ephemeral_from(_invocation: &Invocation) -> bool {
    false
}
