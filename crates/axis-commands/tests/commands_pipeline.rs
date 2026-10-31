//! Command framework tests: declaration, registration payloads, typed
//! invocation parsing, autocomplete, and responses — the response side
//! runs against a mock HTTP server.

use std::sync::{Arc, Mutex};

use axis_commands::{CommandOption, CommandRegistry, FromCommandValue};
use axis_core::{Token, UserId};
use axis_model::{Interaction, InteractionData, InteractionType};
use axis_rest::Http;
use mockito::{Matcher, Server};
use serde_json::{Value, json};

fn http_for(server_url: &str) -> Http {
    Http::builder(Token::new("test-token".to_owned()))
        .api_base(server_url)
        .build()
        .unwrap()
}

/// An application-command interaction for `name` with the given options.
fn command_interaction(id: u64, name: &str, options: Value) -> Interaction {
    serde_json::from_value(json!({
        "id": id.to_string(),
        "application_id": "5",
        "type": 2,
        "data": {
            "id": format!("{}1", id), "name": name,
            "type": 1, "options": options
        },
        "token": "secret-token", "version": 1
    }))
    .unwrap()
}

fn simple_interaction(id: u64, name: &str) -> Interaction {
    command_interaction(id, name, json!([]))
}

#[test]
fn registration_payloads_carry_options_subcommands_and_permissions() {
    let mut registry = CommandRegistry::new();
    registry
        .command("echo", "Repeat after you", |cmd| {
            cmd.option(
                CommandOption::string("text", "What to echo")
                    .required()
                    .max_length(100)
                    .finish(),
            )
            .option(
                CommandOption::integer("times", "How often")
                    .min_value(1.0)
                    .max_value(10.0)
                    .finish(),
            )
            .dm_permission(false)
            .default_member_permissions(axis_core::Permissions::KICK_MEMBERS)
            .handler(|_| async { Ok(()) });
        })
        .command("config", "Configure the bot", |cmd| {
            cmd.subcommand("set", "Set a value", |sub| {
                sub.option(
                    CommandOption::string("key", "Which value")
                        .required()
                        .finish(),
                )
                .handler(|_| async { Ok(()) });
            })
            .group("channel", "Channel settings", |group| {
                group.subcommand("add", "Add a channel", |sub| {
                    sub.option(
                        CommandOption::channel("channel", "The channel")
                            .required()
                            .finish(),
                    )
                    .handler(|_| async { Ok(()) });
                });
            });
        })
        .command("pick", "Pick a preset", |cmd| {
            cmd.option(
                CommandOption::string("preset", "Which preset")
                    .choice("Dog", "animal_dog")
                    .choice("Cat", "animal_cat")
                    .required()
                    .finish(),
            )
            .handler(|_| async { Ok(()) });
        });

    registry.validate().unwrap();
    let fields = registry.to_fields();

    assert_eq!(fields.len(), 3);

    let echo = json!(&fields[0]);
    assert_eq!(echo["name"], "echo");
    assert_eq!(echo["dm_permission"], false);
    assert_eq!(echo["default_member_permissions"], json!("2"));
    assert_eq!(echo["options"][0]["required"], true);
    assert_eq!(echo["options"][0]["max_length"], 100);
    assert_eq!(echo["options"][1]["min_value"], 1.0);

    let config = json!(&fields[1]);
    assert_eq!(config["options"][0]["type"], 1); // subcommand
    assert_eq!(config["options"][0]["options"][0]["name"], "key");
    assert_eq!(config["options"][1]["type"], 2); // subcommand group
    assert_eq!(config["options"][1]["options"][0]["name"], "add");

    let pick = json!(&fields[2]);
    let choices = pick["options"][0]["choices"].as_array().unwrap();
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0]["value"], "animal_dog");
}

#[test]
fn validation_rejects_broken_declarations() {
    let mut missing_handler = CommandRegistry::new();
    missing_handler.command("lonely", "No handler", |cmd| {
        cmd.option(CommandOption::string("x", "X").finish());
    });
    let error = missing_handler.validate().unwrap_err();
    assert!(error.to_string().contains("no handler"));

    let mut required_order = CommandRegistry::new();
    required_order.command("bad", "Bad order", |cmd| {
        cmd.option(CommandOption::string("optional", "Opt").finish())
            .option(CommandOption::string("needed", "Req").required().finish())
            .handler(|_| async { Ok(()) });
    });
    let error = required_order.validate().unwrap_err();
    assert!(error.to_string().contains("required options"));

    let mut bad_name = CommandRegistry::new();
    bad_name.command("Has Spaces", "Invalid name", |cmd| {
        cmd.handler(|_| async { Ok(()) });
    });
    let error = bad_name.validate().unwrap_err();
    assert!(error.to_string().contains("lowercase"));

    let mut choices_and_autocomplete = CommandRegistry::new();
    choices_and_autocomplete.command("auto", "Bad combo", |cmd| {
        cmd.option(
            CommandOption::string("x", "X")
                .choice("fixed", "value")
                .autocomplete(|_| async { Ok(vec![]) })
                .finish(),
        )
        .handler(|_| async { Ok(()) });
    });
    let error = choices_and_autocomplete.validate().unwrap_err();
    assert!(error.to_string().contains("combine choices"));
}

#[tokio::test]
async fn handlers_receive_typed_arguments() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/interactions/100/secret-token/callback")
        .match_body(Matcher::PartialJson(json!({
            "type": 4,
            "data": { "content": "echoed: hello x3" }
        })))
        .with_status(204)
        .create_async()
        .await;

    let mut registry = CommandRegistry::new();
    registry.command("echo", "Repeat", |cmd| {
        cmd.option(
            CommandOption::string("text", "What to echo")
                .required()
                .finish(),
        )
        .option(CommandOption::integer("times", "How often").finish())
        .handler(|invocation| async move {
            let text: String = invocation.arg("text")?;
            let times: i64 = invocation.opt("times")?.unwrap_or(1);
            invocation
                .reply()
                .content(format!("echoed: {text} x{times}"))
                .await?;
            Ok(())
        });
    });

    let http = Arc::new(http_for(&server.url()));
    let interaction = command_interaction(
        100,
        "echo",
        json!([
            { "name": "text", "type": 3, "value": "hello" },
            { "name": "times", "type": 4, "value": 3 }
        ]),
    );
    registry
        .handle(Arc::clone(&http), interaction)
        .await
        .unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn missing_required_argument_is_an_error() {
    let mut registry = CommandRegistry::new();
    registry.command("echo", "Repeat", |cmd| {
        cmd.option(
            CommandOption::string("text", "What to echo")
                .required()
                .finish(),
        )
        .handler(|invocation| async move {
            let _text: String = invocation.arg("text")?;
            Ok(())
        });
    });

    let http = Arc::new(http_for("http://127.0.0.1:9"));
    let error = registry
        .handle(Arc::clone(&http), simple_interaction(1, "echo"))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("missing required option"));

    // And an unknown command is a clear error too.
    let error = registry
        .handle(http, simple_interaction(2, "nope"))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("unknown command"));
}

#[tokio::test]
async fn subcommand_paths_reach_the_right_handler() {
    let hits = Arc::new(Mutex::new(Vec::<String>::new()));

    let mut registry = CommandRegistry::new();
    let set_hits = Arc::clone(&hits);
    let add_hits = Arc::clone(&hits);
    registry.command("config", "Configure", |cmd| {
        cmd.subcommand("set", "Set a value", |sub| {
            sub.option(
                CommandOption::string("key", "Which value")
                    .required()
                    .finish(),
            )
            .handler(move |invocation| {
                let hits = Arc::clone(&set_hits);
                async move {
                    let key: String = invocation.arg("key")?;
                    hits.lock().unwrap().push(format!("set:{key}"));
                    invocation.defer(false).await?;
                    invocation.followup().content("done").await?;
                    Ok(())
                }
            });
        })
        .group("channel", "Channel settings", |group| {
            group.subcommand("add", "Add a channel", |sub| {
                sub.handler(move |_| {
                    let hits = Arc::clone(&add_hits);
                    async move {
                        hits.lock().unwrap().push("channel:add".to_owned());
                        Ok(())
                    }
                });
            });
        });
    });

    let mut server = Server::new_async().await;
    let defer_mock = server
        .mock("POST", "/interactions/300/secret-token/callback")
        .match_body(Matcher::PartialJson(json!({ "type": 5 })))
        .with_status(204)
        .expect(1)
        .create_async()
        .await;
    let followup_mock = server
        .mock("POST", "/webhooks/5/secret-token")
        .match_body(Matcher::PartialJson(json!({ "content": "done" })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "id": "500", "channel_id": "7",
                "author": { "id": "5", "username": "bot", "discriminator": "0" },
                "content": "done", "timestamp": "2022-09-22T14:00:00.436000+00:00",
                "tts": false, "mention_everyone": false, "mentions": [],
                "mention_roles": [], "attachments": [], "embeds": [], "pinned": false, "type": 0
            })
            .to_string(),
        )
        .expect(1)
        .create_async()
        .await;
    let http = Arc::new(http_for(&server.url()));

    // Top-level subcommand: data.options carries the subcommand wrapper.
    let set_interaction = command_interaction(
        300,
        "config",
        json!([
            { "name": "set", "type": 1, "options": [
                { "name": "key", "type": 3, "value": "prefix" }
            ]}
        ]),
    );
    registry
        .handle(Arc::clone(&http), set_interaction)
        .await
        .unwrap();

    // Nested group: group wrapper contains the subcommand wrapper.
    let add_interaction = command_interaction(
        301,
        "config",
        json!([
            { "name": "channel", "type": 2, "options": [
                { "name": "add", "type": 1, "options": [] }
            ]}
        ]),
    );
    registry
        .handle(Arc::clone(&http), add_interaction)
        .await
        .unwrap();

    let hits = hits.lock().unwrap().clone();
    assert_eq!(hits, vec!["set:prefix", "channel:add"]);
    defer_mock.assert_async().await;
    followup_mock.assert_async().await;
}

#[tokio::test]
async fn autocomplete_answers_with_choices() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/interactions/400/secret-token/callback")
        .match_body(Matcher::PartialJson(json!({
            "type": 8,
            "data": { "choices": [ { "name": "Dog", "value": "animal_dog" } ] }
        })))
        .with_status(204)
        .create_async()
        .await;

    let mut registry = CommandRegistry::new();
    registry.command("pet", "Pet something", |cmd| {
        cmd.option(
            CommandOption::string("animal", "Which animal")
                .autocomplete(|invocation| async move {
                    // Inspect the focused input to build suggestions.
                    let typed = invocation.opt::<String>("animal")?.unwrap_or_default();
                    if typed.starts_with('d') {
                        Ok(vec![axis_model::ApplicationCommandOptionChoice {
                            name: "Dog".to_owned(),
                            name_localizations: None,
                            value: json!("animal_dog"),
                        }])
                    } else {
                        Ok(vec![])
                    }
                })
                .finish(),
        )
        .handler(|_| async { Ok(()) });
    });

    let http = Arc::new(http_for(&server.url()));
    let autocomplete_interaction: Interaction = serde_json::from_value(json!({
        "id": "400", "application_id": "5", "type": 4,
        "data": {
            "id": "4001", "name": "pet", "type": 1,
            "options": [ { "name": "animal", "type": 3, "value": "d", "focused": true } ]
        },
        "token": "secret-token", "version": 1
    }))
    .unwrap();
    assert_eq!(
        autocomplete_interaction.r#type,
        InteractionType::ApplicationCommandAutocomplete
    );
    assert!(matches!(
        autocomplete_interaction.data,
        Some(InteractionData::Autocomplete(_))
    ));

    registry
        .handle(Arc::clone(&http), autocomplete_interaction)
        .await
        .unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn non_command_interactions_are_rejected() {
    let mut registry = CommandRegistry::new();
    registry.command("echo", "Repeat", |cmd| {
        cmd.handler(|_| async { Ok(()) });
    });

    let http = Arc::new(http_for("http://127.0.0.1:9"));
    let component: Interaction = serde_json::from_value(json!({
        "id": "1", "application_id": "5", "type": 3,
        "data": { "custom_id": "b", "component_type": 2 },
        "token": "t", "version": 1
    }))
    .unwrap();
    let error = registry.handle(http, component).await.unwrap_err();
    assert!(error.to_string().contains("not commands"));
}

#[test]
fn user_options_resolve_to_typed_ids() {
    // Sanity for the extraction trait used by handlers.
    let value = json!("80351110224678912");
    let user = UserId::from_command_value(&value).unwrap();
    assert_eq!(user.get(), 80_351_110_224_678_912);
}
