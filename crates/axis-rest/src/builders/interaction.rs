//! Interaction response and followup endpoints.
//!
//! Routes in this module embed the interaction token. The token is a
//! credential: it is sent in the URL (as the API requires) but routes
//! carry a sanitized `display` form so errors and logs never contain it.

use axis_core::{
    ApplicationId, InteractionId,
    error::{AxisError, Result},
};
use axis_model::{
    ApplicationCommandOptionChoice, Embed, InteractionCallbackType, InteractionMessageCallbackData,
    Message, MessageFlags,
};

use crate::{client::Http, routing::Route};

/// The literal message ID for editing/deleting an interaction's original
/// response.
pub const ORIGINAL_RESPONSE: &str = "@original";

/// Builder for responding to an interaction with a message
/// (`POST /interactions/{id}/{token}/callback`, type
/// `CHANNEL_MESSAGE_WITH_SOURCE`).
///
/// An initial response is due within 3 seconds of receiving the
/// interaction; defer instead when the handler needs longer.
#[derive(Debug)]
pub struct RespondToInteraction<'a> {
    http: &'a Http,
    interaction_id: InteractionId,
    token: String,
    data: InteractionMessageCallbackData,
}

impl<'a> RespondToInteraction<'a> {
    pub(crate) fn new(
        http: &'a Http,
        interaction_id: InteractionId,
        token: String,
        data: InteractionMessageCallbackData,
    ) -> Self {
        Self {
            http,
            interaction_id,
            token,
            data,
        }
    }

    /// Make the response ephemeral — visible only to the invoking user.
    pub fn ephemeral(mut self, ephemeral: bool) -> Self {
        let mut flags = self.data.flags.unwrap_or_else(MessageFlags::empty);
        flags.set(MessageFlags::EPHEMERAL, ephemeral);
        self.data.flags = Some(flags);
        self
    }

    pub(crate) async fn execute(self) -> Result<()> {
        let body = serde_json::json!({
            "type": InteractionCallbackType::ChannelMessageWithSource,
            "data": self.data,
        });
        let route = callback_route(reqwest::Method::POST, self.interaction_id, &self.token);
        self.http.empty(route, Some(serialize(&body)?)).await
    }
}

into_future!(RespondToInteraction<'a> => Result<()>);

/// Builder for deferring an interaction response
/// (`POST /interactions/{id}/{token}/callback`, type
/// `DEFERRED_CHANNEL_MESSAGE_WITH_SOURCE`).
///
/// Buys ~15 minutes of token validity for a followup; the user sees a
/// loading state until the followup arrives.
#[derive(Debug)]
pub struct DeferInteraction<'a> {
    http: &'a Http,
    interaction_id: InteractionId,
    token: String,
    ephemeral: bool,
}

impl<'a> DeferInteraction<'a> {
    pub(crate) fn new(http: &'a Http, interaction_id: InteractionId, token: String) -> Self {
        Self {
            http,
            interaction_id,
            token,
            ephemeral: false,
        }
    }

    /// Defer ephemerally; the loading state (and the later followup) are
    /// visible only to the invoking user.
    pub fn ephemeral(mut self, ephemeral: bool) -> Self {
        self.ephemeral = ephemeral;
        self
    }

    pub(crate) async fn execute(self) -> Result<()> {
        let data = if self.ephemeral {
            serde_json::json!({ "flags": MessageFlags::EPHEMERAL.bits() })
        } else {
            serde_json::Value::Null
        };
        let body = serde_json::json!({
            "type": InteractionCallbackType::DeferredChannelMessageWithSource,
            "data": data,
        });
        let route = callback_route(reqwest::Method::POST, self.interaction_id, &self.token);
        self.http.empty(route, Some(serialize(&body)?)).await
    }
}

into_future!(DeferInteraction<'a> => Result<()>);

/// Builder for answering an autocomplete request
/// (`POST /interactions/{id}/{token}/callback`, type
/// `APPLICATION_COMMAND_AUTOCOMPLETE_RESULT`).
///
/// Must be sent within the interaction window; up to 25 choices.
#[derive(Debug)]
pub struct RespondToAutocomplete<'a> {
    http: &'a Http,
    interaction_id: InteractionId,
    token: String,
    choices: Vec<ApplicationCommandOptionChoice>,
}

impl<'a> RespondToAutocomplete<'a> {
    pub(crate) fn new(
        http: &'a Http,
        interaction_id: InteractionId,
        token: String,
        choices: Vec<ApplicationCommandOptionChoice>,
    ) -> Self {
        Self {
            http,
            interaction_id,
            token,
            choices,
        }
    }

    pub(crate) async fn execute(self) -> Result<()> {
        if self.choices.len() > 25 {
            return Err(AxisError::invalid_data(
                "autocomplete responses are limited to 25 choices",
            ));
        }
        let body = serde_json::json!({
            "type": InteractionCallbackType::ApplicationCommandAutocompleteResult,
            "data": { "choices": self.choices },
        });
        let route = callback_route(reqwest::Method::POST, self.interaction_id, &self.token);
        self.http.empty(route, Some(serialize(&body)?)).await
    }
}

into_future!(RespondToAutocomplete<'a> => Result<()>);

/// Builder for sending a followup message for a (usually deferred)
/// interaction (`POST /webhooks/{application_id}/{interaction.token}`).
#[derive(Debug)]
pub struct CreateFollowup<'a> {
    http: &'a Http,
    application_id: ApplicationId,
    token: String,
    data: InteractionMessageCallbackData,
    files: Vec<crate::client::FileUpload>,
}

impl<'a> CreateFollowup<'a> {
    pub(crate) fn new(
        http: &'a Http,
        application_id: ApplicationId,
        token: String,
        data: InteractionMessageCallbackData,
    ) -> Self {
        Self {
            http,
            application_id,
            token,
            data,
            files: Vec::new(),
        }
    }

    /// Attach a file (images preview inline).
    pub fn file(mut self, filename: impl Into<String>, bytes: Vec<u8>) -> Self {
        self.files.push(crate::client::FileUpload {
            filename: filename.into(),
            bytes,
        });
        self
    }

    /// Make the followup ephemeral — visible only to the invoking user.
    pub fn ephemeral(mut self, ephemeral: bool) -> Self {
        let mut flags = self.data.flags.unwrap_or_else(MessageFlags::empty);
        flags.set(MessageFlags::EPHEMERAL, ephemeral);
        self.data.flags = Some(flags);
        self
    }

    pub(crate) async fn execute(self) -> Result<Message> {
        let route = webhook_route(
            reqwest::Method::POST,
            format!("/webhooks/{}/{}", self.application_id, self.token),
            self.application_id,
            "",
        );
        if self.files.is_empty() {
            return self.http.json(route, Some(serialize(&self.data)?), None).await;
        }
        let mut body: serde_json::Value = serde_json::from_slice(&serialize(&self.data)?)
            .map_err(|error| AxisError::Serialization(error.into()))?;
        body["attachments"] = serde_json::Value::Array(
            self.files
                .iter()
                .enumerate()
                .map(|(id, file)| serde_json::json!({ "id": id, "filename": file.filename }))
                .collect(),
        );
        let body = serde_json::to_vec(&body).map_err(|error| AxisError::Serialization(error.into()))?;
        self.http.json_with_files(route, body, self.files).await
    }
}

into_future!(CreateFollowup<'a> => Result<Message>);

/// Editable fields of a followup message. `clear_content` /
/// `clear_embeds` send explicit nulls, which Discord interprets as
/// removal.
#[derive(Debug, Default)]
pub struct FollowupEditFields {
    pub(crate) content: Option<Option<String>>,
    pub(crate) embeds: Option<Option<Vec<Embed>>>,
    pub(crate) flags: Option<MessageFlags>,
}

impl FollowupEditFields {
    /// Set new content.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(Some(content.into()));
        self
    }

    /// Remove the content (sends `null`).
    pub fn clear_content(mut self) -> Self {
        self.content = Some(None);
        self
    }

    /// Replace the embeds.
    pub fn embeds(mut self, embeds: impl IntoIterator<Item = Embed>) -> Self {
        self.embeds = Some(Some(embeds.into_iter().collect()));
        self
    }

    /// Remove all embeds (sends `null`).
    pub fn clear_embeds(mut self) -> Self {
        self.embeds = Some(None);
        self
    }

    /// Set message flags (e.g. suppress embeds).
    pub fn flags(mut self, flags: MessageFlags) -> Self {
        self.flags = Some(flags);
        self
    }

    fn to_json(&self) -> serde_json::Value {
        #[derive(serde::Serialize)]
        struct Body {
            #[serde(skip_serializing_if = "Option::is_none")]
            content: Option<Option<String>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            embeds: Option<Option<Vec<Embed>>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            flags: Option<MessageFlags>,
        }
        serde_json::to_value(Body {
            content: self.content.clone(),
            embeds: self.embeds.clone(),
            flags: self.flags,
        })
        .unwrap_or(serde_json::Value::Null)
    }
}

/// Builder for editing a followup message
/// (`PATCH /webhooks/{application_id}/{interaction.token}/messages/{message_id}`).
///
/// Pass the literal [`ORIGINAL_RESPONSE`] to edit the response that
/// started the interaction, or a followup message's ID.
#[derive(Debug)]
pub struct EditFollowup<'a> {
    http: &'a Http,
    application_id: ApplicationId,
    token: String,
    message_id: String,
    fields: FollowupEditFields,
}

impl<'a> EditFollowup<'a> {
    pub(crate) fn new(
        http: &'a Http,
        application_id: ApplicationId,
        token: String,
        message_id: impl core::fmt::Display,
        fields: FollowupEditFields,
    ) -> Self {
        Self {
            http,
            application_id,
            token,
            message_id: message_id.to_string(),
            fields,
        }
    }

    pub(crate) async fn execute(self) -> Result<Message> {
        let route = webhook_route(
            reqwest::Method::PATCH,
            format!(
                "/webhooks/{}/{}/messages/{}",
                self.application_id, self.token, self.message_id
            ),
            self.application_id,
            "messages/{message_id}",
        );
        self.http
            .json(route, Some(serialize(&self.fields.to_json())?), None)
            .await
    }
}

into_future!(EditFollowup<'a> => Result<Message>);

/// Builder for deleting a followup message
/// (`DELETE /webhooks/{application_id}/{interaction.token}/messages/{message_id}`).
///
/// Pass the literal [`ORIGINAL_RESPONSE`] to delete the response that
/// started the interaction, or a followup message's ID.
#[derive(Debug)]
pub struct DeleteFollowup<'a> {
    http: &'a Http,
    application_id: ApplicationId,
    token: String,
    message_id: String,
}

impl<'a> DeleteFollowup<'a> {
    pub(crate) fn new(
        http: &'a Http,
        application_id: ApplicationId,
        token: String,
        message_id: impl core::fmt::Display,
    ) -> Self {
        Self {
            http,
            application_id,
            token,
            message_id: message_id.to_string(),
        }
    }

    pub(crate) async fn execute(self) -> Result<()> {
        let route = webhook_route(
            reqwest::Method::DELETE,
            format!(
                "/webhooks/{}/{}/messages/{}",
                self.application_id, self.token, self.message_id
            ),
            self.application_id,
            "messages/{message_id}",
        );
        self.http.empty(route, None).await
    }
}

into_future!(DeleteFollowup<'a> => Result<()>);

/// Build the interaction callback route with a sanitized display path.
fn callback_route(method: reqwest::Method, interaction_id: InteractionId, token: &str) -> Route {
    let path = format!("/interactions/{interaction_id}/{token}/callback");
    Route::new(
        method,
        "/interactions/{interaction_id}/{token}/callback",
        path,
    )
    .displaying(format!("/interactions/{interaction_id}/[token]/callback"))
}

/// Build a webhook (followup) route with a sanitized display path.
/// `display_suffix` appears in the display form so routes stay
/// distinguishable without leaking the token.
fn webhook_route(
    method: reqwest::Method,
    path: String,
    application_id: ApplicationId,
    display_suffix: &str,
) -> Route {
    Route::new(
        method,
        "/webhooks/{application_id}/{interaction_token}",
        path,
    )
    .displaying(format!(
        "/webhooks/{application_id}/[token]{display_suffix}"
    ))
}

fn serialize<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|error| AxisError::Serialization(error.into()))
}
