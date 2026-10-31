//! Tests for application command registration and interaction responses
//! against a local mock HTTP server.

use axis_core::{
    ApplicationCommandId, ApplicationId, GuildId, InteractionId, MessageId, Permissions, Token,
    error::AxisError,
};
use axis_model::{
    ApplicationCommandFields, ApplicationCommandOption, ApplicationCommandOptionChoice,
    ApplicationCommandOptionType, ApplicationCommandPermission,
};
use axis_rest::{Http, ORIGINAL_RESPONSE};
use mockito::{Matcher, Server};
use serde_json::json;

fn http_for(server_url: &str) -> Http {
    Http::builder(Token::new("test-token".to_owned()))
        .api_base(server_url)
        .build()
        .unwrap()
}

fn text_option() -> ApplicationCommandOption {
    ApplicationCommandOption {
        r#type: ApplicationCommandOptionType::String,
        name: "text".to_owned(),
        description: "What to echo".to_owned(),
        required: Some(true),
        choices: None,
        options: None,
        channel_types: None,
        autocomplete: None,
        min_value: None,
        max_value: None,
        min_length: None,
        max_length: None,
    }
}

fn echo_command_fields() -> ApplicationCommandFields {
    ApplicationCommandFields {
        name: Some("echo".to_owned()),
        description: Some("Repeats text".to_owned()),
        options: Some(vec![text_option()]),
        ..ApplicationCommandFields::default()
    }
}

#[tokio::test]
async fn bulk_overwrite_global_commands() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("PUT", "/applications/5/commands")
        .match_header("authorization", "Bot test-token")
        .match_body(Matcher::PartialJson(json!([
            { "name": "echo", "description": "Repeats text" }
        ])))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!([{
                "id": "42", "application_id": "5", "name": "echo",
                "description": "Repeats text", "default_member_permissions": null,
                "version": "43"
            }])
            .to_string(),
        )
        .create_async()
        .await;

    let http = http_for(&server.url());
    let commands = http
        .bulk_overwrite_global_commands(ApplicationId::new(5), vec![echo_command_fields()])
        .await
        .unwrap();

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id.get(), 42);
    assert_eq!(commands[0].name, "echo");
    mock.assert_async().await;
}

#[tokio::test]
async fn bulk_overwrite_guild_commands_hits_guild_route() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("PUT", "/applications/5/guilds/9/commands")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json!([]).to_string())
        .create_async()
        .await;

    let http = http_for(&server.url());
    let commands = http
        .bulk_overwrite_guild_commands(ApplicationId::new(5), GuildId::new(9), vec![])
        .await
        .unwrap();
    assert!(commands.is_empty());
    mock.assert_async().await;
}

#[tokio::test]
async fn create_command_validates_name_length() {
    let http = Http::builder(Token::new("t".to_owned()))
        .api_base("http://127.0.0.1:9")
        .build()
        .unwrap();
    let mut fields = echo_command_fields();
    fields.name = Some("x".repeat(33));
    let error = http
        .create_global_command(ApplicationId::new(5), fields)
        .await
        .expect_err("overlong names must be rejected locally");
    assert!(error.to_string().contains("1-32"));
}

#[tokio::test]
async fn command_permissions_roundtrip() {
    let mut server = Server::new_async().await;
    let get_mock = server
        .mock("GET", "/applications/5/guilds/9/commands/permissions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!([{
                "id": "42", "application_id": "5", "guild_id": "9",
                "permissions": [ { "id": "41771983423143937", "type": 1, "permission": true } ]
            }])
            .to_string(),
        )
        .create_async()
        .await;
    let put_mock = server
        .mock("PUT", "/applications/5/guilds/9/commands/42/permissions")
        .match_body(Matcher::PartialJson(json!({
            "permissions": [ { "id": 41771983423143937_u64, "type": 1, "permission": true } ]
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "id": "42", "application_id": "5", "guild_id": "9",
                "permissions": [ { "id": "41771983423143937", "type": 1, "permission": true } ]
            })
            .to_string(),
        )
        .create_async()
        .await;

    let http = http_for(&server.url());
    let all = http
        .get_guild_command_permissions(ApplicationId::new(5), GuildId::new(9))
        .await
        .unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].permissions.len(), 1);

    let updated = http
        .set_guild_command_permissions(
            ApplicationId::new(5),
            GuildId::new(9),
            ApplicationCommandId::new(42),
            vec![ApplicationCommandPermission {
                id: axis_core::GenericId::new(41_771_983_423_143_937),
                r#type: axis_model::ApplicationCommandPermissionType::Role,
                permission: true,
            }],
        )
        .await
        .unwrap();
    assert_eq!(updated.id.get(), 42);
    get_mock.assert_async().await;
    put_mock.assert_async().await;
}

#[tokio::test]
async fn interaction_callback_sends_type_and_ephemeral_flag() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/interactions/100/secret-token/callback")
        .match_body(Matcher::PartialJson(json!({
            "type": 4,
            "data": { "content": "hi there", "flags": 64 }
        })))
        .with_status(204)
        .create_async()
        .await;

    let http = http_for(&server.url());
    http.respond_to_interaction(
        InteractionId::new(100),
        "secret-token",
        axis_model::InteractionMessageCallbackData {
            content: Some("hi there".to_owned()),
            ..Default::default()
        },
    )
    .ephemeral(true)
    .await
    .unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn defer_sends_type_5_with_ephemeral_data() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/interactions/100/secret-token/callback")
        .match_body(Matcher::PartialJson(json!({
            "type": 5,
            "data": { "flags": 64 }
        })))
        .with_status(204)
        .create_async()
        .await;

    let http = http_for(&server.url());
    http.defer_interaction(InteractionId::new(100), "secret-token")
        .ephemeral(true)
        .await
        .unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn autocomplete_response_carries_choices() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/interactions/100/secret-token/callback")
        .match_body(Matcher::PartialJson(json!({
            "type": 8,
            "data": { "choices": [ { "name": "Dog", "value": "animal_dog" } ] }
        })))
        .with_status(204)
        .create_async()
        .await;

    let http = http_for(&server.url());
    http.respond_to_autocomplete(
        InteractionId::new(100),
        "secret-token",
        vec![ApplicationCommandOptionChoice {
            name: "Dog".to_owned(),
            name_localizations: None,
            value: json!("animal_dog"),
        }],
    )
    .await
    .unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn followups_use_the_webhook_routes() {
    let mut server = Server::new_async().await;
    let create = server
        .mock("POST", "/webhooks/5/secret-token")
        .match_body(Matcher::PartialJson(json!({ "content": "followup!" })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "id": "777", "channel_id": "7",
                "author": { "id": "5", "username": "bot", "discriminator": "0" },
                "content": "followup!", "timestamp": "2022-09-22T14:00:00.436000+00:00",
                "tts": false, "mention_everyone": false, "mentions": [],
                "mention_roles": [], "attachments": [], "embeds": [], "pinned": false, "type": 0
            })
            .to_string(),
        )
        .create_async()
        .await;
    let edit = server
        .mock("PATCH", "/webhooks/5/secret-token/messages/@original")
        .match_body(Matcher::PartialJson(json!({ "content": "edited" })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "id": "500", "channel_id": "7",
                "author": { "id": "5", "username": "bot", "discriminator": "0" },
                "content": "edited", "timestamp": "2022-09-22T14:00:00.436000+00:00",
                "tts": false, "mention_everyone": false, "mentions": [],
                "mention_roles": [], "attachments": [], "embeds": [], "pinned": false, "type": 0
            })
            .to_string(),
        )
        .create_async()
        .await;
    let delete = server
        .mock("DELETE", "/webhooks/5/secret-token/messages/777")
        .with_status(204)
        .create_async()
        .await;

    let http = http_for(&server.url());
    let message = http
        .create_followup(
            ApplicationId::new(5),
            "secret-token",
            axis_model::InteractionMessageCallbackData {
                content: Some("followup!".to_owned()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(message.id.get(), 777);

    let edited = http
        .edit_followup(
            ApplicationId::new(5),
            "secret-token",
            ORIGINAL_RESPONSE,
            axis_rest::builders::FollowupEditFields::default().content("edited"),
        )
        .await
        .unwrap();
    assert_eq!(edited.content, "edited");

    http.delete_followup(ApplicationId::new(5), "secret-token", MessageId::new(777))
        .await
        .unwrap();

    create.assert_async().await;
    edit.assert_async().await;
    delete.assert_async().await;
}

#[tokio::test]
async fn interaction_tokens_never_leak_into_errors() {
    let mut server = Server::new_async().await;
    server
        .mock("POST", "/interactions/100/secret-token/callback")
        .with_status(403)
        .with_body(json!({ "code": 50_045, "message": "no permissions" }).to_string())
        .create_async()
        .await;

    let http = http_for(&server.url());
    let error = http
        .respond_to_interaction(
            InteractionId::new(100),
            "secret-token",
            axis_model::InteractionMessageCallbackData {
                content: Some("hi".to_owned()),
                ..Default::default()
            },
        )
        .await
        .expect_err("403 must fail");

    let AxisError::Http(http_error) = error else {
        panic!("expected Http error, got {error:?}")
    };
    assert!(
        !http_error.route.contains("secret-token"),
        "the interaction token must be sanitized out of error routes: {}",
        http_error.route
    );
    assert!(http_error.route.contains("[token]"));
}

#[test]
fn default_member_permissions_serialize_as_string() {
    let fields = ApplicationCommandFields {
        name: Some("x".to_owned()),
        description: Some("y".to_owned()),
        default_member_permissions: Some(Permissions::KICK_MEMBERS),
        ..ApplicationCommandFields::default()
    };
    let json = serde_json::to_value(&fields).unwrap();
    // Permission bitsets go over the wire as decimal strings.
    assert_eq!(json["default_member_permissions"], json!("2"));
}
