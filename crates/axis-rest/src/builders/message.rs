//! Message endpoints: create, edit, delete, and retrieve messages.

use serde::Serialize;
use serde_json::json;

use axis_core::{
    ChannelId, MessageId,
    error::{AxisError, Result},
};
use axis_model::{AllowedMentions, Embed, Message, MessageFlags, MessageReference, PollCreate};

use crate::{client::Http, routing::Route};

/// Builder for sending a message to a channel
/// (`POST /channels/{id}/messages`).
///
/// A message must have non-empty `content` or at least one embed; content
/// is capped at 2000 characters. These constraints are validated locally
/// before any request is sent.
#[derive(Debug)]
pub struct CreateMessage<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    fields: CreateMessageFields,
    files: Vec<crate::client::FileUpload>,
}

#[derive(Debug, Default, Serialize)]
struct CreateMessageFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embeds: Option<Vec<Embed>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_reference: Option<MessageReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<MessageFlags>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_mentions: Option<AllowedMentions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    poll: Option<PollCreate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    components: Option<Vec<serde_json::Value>>,
}

impl<'a> CreateMessage<'a> {
    /// Add message components (action rows of buttons and select menus),
    /// as raw component JSON.
    pub fn components(mut self, components: Vec<serde_json::Value>) -> Self {
        self.fields.components = Some(components);
        self
    }

    pub(crate) fn new(http: &'a Http, channel_id: ChannelId) -> Self {
        Self {
            http,
            channel_id,
            fields: CreateMessageFields::default(),
            files: Vec::new(),
        }
    }

    /// Attach a file (up to 10 per message; images preview inline).
    pub fn file(mut self, filename: impl Into<String>, bytes: Vec<u8>) -> Self {
        self.files.push(crate::client::FileUpload {
            filename: filename.into(),
            bytes,
        });
        self
    }

    /// Set the message content (up to 2000 characters).
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.fields.content = Some(content.into());
        self
    }

    /// Send the message with text-to-speech.
    pub fn tts(mut self, tts: bool) -> Self {
        self.fields.tts = Some(tts);
        self
    }

    /// Set the embeds to attach (up to 10; Discord rejects more).
    pub fn embeds(mut self, embeds: impl IntoIterator<Item = Embed>) -> Self {
        self.fields.embeds = Some(embeds.into_iter().collect());
        self
    }

    /// Make the message a reply to another message in the same channel.
    pub fn reply(mut self, message_id: MessageId) -> Self {
        self.fields.message_reference = Some(MessageReference {
            message_id: Some(message_id),
            ..MessageReference::default()
        });
        self
    }

    /// Set the raw message reference (for forwards or cross-channel
    /// replies).
    pub fn reference(mut self, reference: MessageReference) -> Self {
        self.fields.message_reference = Some(reference);
        self
    }

    /// Set message flags. Only a subset of flags may be set when sending
    /// (e.g. [`MessageFlags::SUPPRESS_EMBEDS`],
    /// [`MessageFlags::SUPPRESS_NOTIFICATIONS`]).
    pub fn flags(mut self, flags: MessageFlags) -> Self {
        self.fields.flags = Some(flags);
        self
    }

    /// Restrict which mentions in the content ping (see
    /// [`AllowedMentions`]). Without this, Discord pings everything the
    /// content mentions that the bot is allowed to.
    pub fn allowed_mentions(mut self, allowed: AllowedMentions) -> Self {
        self.fields.allowed_mentions = Some(allowed);
        self
    }

    /// Attach a poll.
    pub fn poll(mut self, poll: PollCreate) -> Self {
        self.fields.poll = Some(poll);
        self
    }

    pub(crate) async fn execute(self) -> Result<Message> {
        let content_chars = self
            .fields
            .content
            .as_ref()
            .map_or(0, |content| content.chars().count());
        if content_chars > 2000 {
            return Err(AxisError::invalid_data(format!(
                "message content is {content_chars} characters; Discord allows at most 2000"
            )));
        }
        if let Some(poll) = &self.fields.poll {
            if !(1..=10).contains(&poll.answers.len()) {
                return Err(AxisError::invalid_data("a poll needs 1-10 answers"));
            }
            if !(1..=768).contains(&poll.duration) {
                return Err(AxisError::invalid_data("poll duration must be 1-768 hours"));
            }
        }
        if self.files.len() > 10 {
            return Err(AxisError::invalid_data(
                "a message can carry at most 10 files",
            ));
        }
        if content_chars == 0
            && self.files.is_empty()
            && self.fields.poll.is_none()
            && self
                .fields
                .embeds
                .as_ref()
                .is_none_or(|embeds| embeds.is_empty())
        {
            return Err(AxisError::invalid_data(
                "a message must have content or at least one embed",
            ));
        }

        let mut fields = self.fields;
        if !self.files.is_empty() {
            fields.attachments = Some(
                self.files
                    .iter()
                    .enumerate()
                    .map(|(id, file)| json!({ "id": id, "filename": file.filename }))
                    .collect(),
            );
        }
        let body =
            serde_json::to_vec(&fields).map_err(|error| AxisError::Serialization(error.into()))?;
        let route = Route::with_major(
            reqwest::Method::POST,
            "/channels/{channel_id}/messages",
            format!("/channels/{}/messages", self.channel_id),
            self.channel_id.get(),
        );
        if self.files.is_empty() {
            self.http.json(route, Some(body), None).await
        } else {
            self.http.json_with_files(route, body, self.files).await
        }
    }
}

into_future!(CreateMessage<'a> => Result<Message>);

/// Builder for editing a message (`PATCH /channels/{id}/messages/{id}`).
///
/// Fields that are never set are left unchanged. `clear_content` and
/// `clear_embeds` send explicit nulls, which Discord interprets as
/// removing the content.
#[derive(Debug)]
pub struct EditMessage<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    message_id: MessageId,
    fields: EditMessageFields,
    files: Vec<crate::client::FileUpload>,
}

#[derive(Debug, Default, Serialize)]
struct EditMessageFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embeds: Option<Option<Vec<Embed>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<MessageFlags>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_mentions: Option<AllowedMentions>,
    /// Sent ONLY when explicitly managed (clear_attachments); omitting it
    /// keeps every attachment the message already has.
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<serde_json::Value>>,
}

impl<'a> EditMessage<'a> {
    pub(crate) fn new(http: &'a Http, channel_id: ChannelId, message_id: MessageId) -> Self {
        Self {
            http,
            channel_id,
            message_id,
            fields: EditMessageFields::default(),
            files: Vec::new(),
        }
    }

    /// Attach a file to the message with this edit (up to 10 total).
    ///
    /// Files sent here are ADDED; attachments already on the message are
    /// kept unless [`Self::clear_attachments`] is called. A content-only
    /// edit (no files, no `clear_attachments`) never removes existing
    /// attachments.
    pub fn file(mut self, filename: impl Into<String>, bytes: Vec<u8>) -> Self {
        self.files.push(crate::client::FileUpload {
            filename: filename.into(),
            bytes,
        });
        self
    }

    /// Drop the attachments already on the message (they are kept by
    /// default, including on content-only edits).
    pub fn clear_attachments(mut self) -> Self {
        self.fields.attachments = Some(vec![]);
        self
    }

    /// Set new message content (up to 2000 characters).
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.fields.content = Some(Some(content.into()));
        self
    }

    /// Remove the message content entirely (sends `null`).
    pub fn clear_content(mut self) -> Self {
        self.fields.content = Some(None);
        self
    }

    /// Replace the message's embeds.
    pub fn embeds(mut self, embeds: impl IntoIterator<Item = Embed>) -> Self {
        self.fields.embeds = Some(Some(embeds.into_iter().collect()));
        self
    }

    /// Remove all embeds (sends `null`).
    pub fn clear_embeds(mut self) -> Self {
        self.fields.embeds = Some(None);
        self
    }

    /// Set message flags; only [`MessageFlags::SUPPRESS_EMBEDS`] may be
    /// toggled after the fact.
    pub fn flags(mut self, flags: MessageFlags) -> Self {
        self.fields.flags = Some(flags);
        self
    }

    /// Restrict which mentions in the new content ping.
    pub fn allowed_mentions(mut self, allowed: AllowedMentions) -> Self {
        self.fields.allowed_mentions = Some(allowed);
        self
    }

    pub(crate) async fn execute(self) -> Result<Message> {        let mut fields = self.fields;
        if !self.files.is_empty() {
            // Describe the new files so Discord attaches them; the ids
            // index into THIS request's files[N] parts. Existing
            // attachments are kept (we never send an empty array).
            let described: Vec<serde_json::Value> = self
                .files
                .iter()
                .enumerate()
                .map(|(index, file)| {
                    serde_json::json!({
                        "id": index,
                        "filename": file.filename,
                    })
                })
                .collect();
            fields.attachments = Some(described);
        }
        let route = Route::with_major(
            reqwest::Method::PATCH,
            "/channels/{channel_id}/messages/{message_id}",
            format!(
                "/channels/{}/messages/{}",
                self.channel_id, self.message_id
            ),
            self.channel_id.get(),
        );
        if self.files.is_empty() {
            let body = serde_json::to_vec(&fields)
                .map_err(|error| AxisError::Serialization(error.into()))?;
            self.http.json(route, Some(body), None).await
        } else {
            let json = serde_json::to_vec(&fields)
                .map_err(|error| AxisError::Serialization(error.into()))?;
            self.http
                .json_with_files(route, json, self.files)
                .await
        }
    }
}

into_future!(EditMessage<'a> => Result<Message>);

/// Builder for deleting a message
/// (`DELETE /channels/{id}/messages/{id}`).
#[derive(Debug)]
pub struct DeleteMessage<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    message_id: MessageId,
    reason: Option<String>,
}

impl<'a> DeleteMessage<'a> {
    pub(crate) fn new(http: &'a Http, channel_id: ChannelId, message_id: MessageId) -> Self {
        Self {
            http,
            channel_id,
            message_id,
            reason: None,
        }
    }

    /// Record a reason in the guild's audit log (for deleting others'
    /// messages).
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    pub(crate) async fn execute(self) -> Result<()> {
        let route = Route::with_major(
            reqwest::Method::DELETE,
            "/channels/{channel_id}/messages/{message_id}",
            format!("/channels/{}/messages/{}", self.channel_id, self.message_id),
            self.channel_id.get(),
        )
        .with_reason(self.reason);
        self.http.empty(route, None).await
    }
}

into_future!(DeleteMessage<'a> => Result<()>);

/// Builder for fetching a single message
/// (`GET /channels/{id}/messages/{id}`).
#[derive(Debug)]
pub struct GetMessage<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    message_id: MessageId,
}

impl<'a> GetMessage<'a> {
    pub(crate) fn new(http: &'a Http, channel_id: ChannelId, message_id: MessageId) -> Self {
        Self {
            http,
            channel_id,
            message_id,
        }
    }

    pub(crate) async fn execute(self) -> Result<Message> {
        let route = Route::with_major(
            reqwest::Method::GET,
            "/channels/{channel_id}/messages/{message_id}",
            format!("/channels/{}/messages/{}", self.channel_id, self.message_id),
            self.channel_id.get(),
        );
        self.http.json(route, None, None).await
    }
}

into_future!(GetMessage<'a> => Result<Message>);

/// Builder for listing a channel's messages
/// (`GET /channels/{id}/messages`).
///
/// Without parameters this returns the most recent 50 messages. `limit`
/// may be 1–100; `around`, `before`, and `after` paginate relative to a
/// message ID (`around` excludes the others).
#[derive(Debug)]
pub struct GetMessages<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    limit: Option<u8>,
    before: Option<MessageId>,
    after: Option<MessageId>,
    around: Option<MessageId>,
}

impl<'a> GetMessages<'a> {
    pub(crate) fn new(http: &'a Http, channel_id: ChannelId) -> Self {
        Self {
            http,
            channel_id,
            limit: None,
            before: None,
            after: None,
            around: None,
        }
    }

    /// Maximum number of messages to return (1–100).
    pub fn limit(mut self, limit: u8) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Messages before this ID (newest first).
    pub fn before(mut self, message_id: MessageId) -> Self {
        self.before = Some(message_id);
        self
    }

    /// Messages after this ID (oldest first).
    pub fn after(mut self, message_id: MessageId) -> Self {
        self.after = Some(message_id);
        self
    }

    /// Messages around this ID. Mutually exclusive with `before`/`after`.
    pub fn around(mut self, message_id: MessageId) -> Self {
        self.around = Some(message_id);
        self
    }

    pub(crate) async fn execute(self) -> Result<Vec<Message>> {
        if self.limit.is_some_and(|limit| limit == 0 || limit > 100) {
            return Err(AxisError::invalid_data(
                "message list limit must be between 1 and 100",
            ));
        }
        if self.around.is_some() && (self.before.is_some() || self.after.is_some()) {
            return Err(AxisError::invalid_data(
                "`around` cannot be combined with `before` or `after`",
            ));
        }

        let mut query: Vec<(&'static str, String)> = Vec::new();
        if let Some(limit) = self.limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(before) = self.before {
            query.push(("before", before.to_string()));
        }
        if let Some(after) = self.after {
            query.push(("after", after.to_string()));
        }
        if let Some(around) = self.around {
            query.push(("around", around.to_string()));
        }
        let query = if query.is_empty() { None } else { Some(query) };

        let route = Route::with_major(
            reqwest::Method::GET,
            "/channels/{channel_id}/messages",
            format!("/channels/{}/messages", self.channel_id),
            self.channel_id.get(),
        );
        self.http.json(route, None, query.as_deref()).await
    }
}

into_future!(GetMessages<'a> => Result<Vec<Message>>);

/// Builder for bulk-deleting 2–100 messages in one request
/// (`POST /channels/{id}/messages/bulk-delete`).
///
/// Only works in guild channels; messages must be younger than two weeks.
#[derive(Debug)]
pub struct BulkDeleteMessages<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    message_ids: Vec<MessageId>,
}

impl<'a> BulkDeleteMessages<'a> {
    pub(crate) fn new(http: &'a Http, channel_id: ChannelId, message_ids: Vec<MessageId>) -> Self {
        Self {
            http,
            channel_id,
            message_ids,
        }
    }

    pub(crate) async fn execute(self) -> Result<()> {
        if !(2..=100).contains(&self.message_ids.len()) {
            return Err(AxisError::invalid_data(format!(
                "bulk delete requires between 2 and 100 messages, got {}",
                self.message_ids.len()
            )));
        }

        let body = serde_json::to_vec(&json!({
            "messages": self.message_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
        }))
        .map_err(|error| AxisError::Serialization(error.into()))?;
        let route = Route::with_major(
            reqwest::Method::POST,
            "/channels/{channel_id}/messages/bulk-delete",
            format!("/channels/{}/messages/bulk-delete", self.channel_id),
            self.channel_id.get(),
        );
        self.http.empty(route, Some(body)).await
    }
}

into_future!(BulkDeleteMessages<'a> => Result<()>);

#[cfg(test)]
mod tests {
    use super::*;

    /// Local validation must reject invalid requests before any HTTP call.
    #[tokio::test]
    async fn create_message_validates_empty_content() {
        let http = crate::test_support::http_for_tests();
        let error = http
            .create_message(ChannelId::new(1))
            .await
            .expect_err("empty message must be rejected");
        assert!(error.to_string().contains("content or at least one embed"));
    }

    #[tokio::test]
    async fn create_message_validates_length() {
        let http = crate::test_support::http_for_tests();
        let error = http
            .create_message(ChannelId::new(1))
            .content("x".repeat(2001))
            .await
            .expect_err("overlong content must be rejected");
        assert!(error.to_string().contains("2000"));
    }

    #[tokio::test]
    async fn get_messages_validates_limit() {
        let http = crate::test_support::http_for_tests();
        let error = http
            .get_messages(ChannelId::new(1))
            .limit(101)
            .await
            .expect_err("limit over 100 must be rejected");
        assert!(error.to_string().contains("1 and 100"));

        let error = http
            .get_messages(ChannelId::new(1))
            .limit(0)
            .await
            .expect_err("limit 0 must be rejected");
        assert!(error.to_string().contains("1 and 100"));
    }

    #[tokio::test]
    async fn get_messages_validates_around_exclusivity() {
        let http = crate::test_support::http_for_tests();
        let error = http
            .get_messages(ChannelId::new(1))
            .around(MessageId::new(10))
            .before(MessageId::new(20))
            .await
            .expect_err("around+before must be rejected");
        assert!(error.to_string().contains("around"));
    }

    #[test]
    fn create_fields_serialize_without_nulls() {
        let fields = CreateMessageFields {
            content: Some("hi".to_owned()),
            ..CreateMessageFields::default()
        };
        let json = serde_json::to_value(&fields).unwrap();
        assert_eq!(json, json!({ "content": "hi" }));
    }

    #[test]
    fn edit_message_null_clears_serialize() {
        let fields = EditMessageFields {
            content: Some(None),
            embeds: Some(Some(vec![])),
            ..EditMessageFields::default()
        };
        let json = serde_json::to_value(&fields).unwrap();
        assert_eq!(json, json!({ "content": null, "embeds": [] }));
    }
}
