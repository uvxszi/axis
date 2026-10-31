//! Message reaction endpoints.
//!
//! Note: Discord rate-limits reaction endpoints per-guild with quotas that
//! may be inaccurate — 429s can still occur even when the limiter thinks
//! there is capacity; the pipeline handles them by waiting and retrying.

use axis_core::{ChannelId, MessageId, UserId, error::Result};
use axis_model::Emoji;

use crate::{client::Http, routing::Route};

/// Builder for reacting to a message with the bot's own account
/// (`PUT /channels/{id}/messages/{id}/reactions/{emoji}/@me`).
#[derive(Debug)]
pub struct AddReaction<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    message_id: MessageId,
    emoji: Emoji,
}

impl<'a> AddReaction<'a> {
    pub(crate) fn new(
        http: &'a Http,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: Emoji,
    ) -> Self {
        Self {
            http,
            channel_id,
            message_id,
            emoji,
        }
    }

    pub(crate) async fn execute(self) -> Result<()> {
        let route = Route::with_major(
            reqwest::Method::PUT,
            "/channels/{channel_id}/messages/{message_id}/reactions/{emoji}/@me",
            format!(
                "/channels/{}/messages/{}/reactions/{}/@me",
                self.channel_id,
                self.message_id,
                self.emoji.url_part()
            ),
            self.channel_id.get(),
        );
        self.http.empty(route, None).await
    }
}

into_future!(AddReaction<'a> => Result<()>);

/// Builder for removing the bot's own reaction
/// (`DELETE /channels/{id}/messages/{id}/reactions/{emoji}/@me`).
#[derive(Debug)]
pub struct DeleteOwnReaction<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    message_id: MessageId,
    emoji: Emoji,
}

impl<'a> DeleteOwnReaction<'a> {
    pub(crate) fn new(
        http: &'a Http,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: Emoji,
    ) -> Self {
        Self {
            http,
            channel_id,
            message_id,
            emoji,
        }
    }

    pub(crate) async fn execute(self) -> Result<()> {
        let route = Route::with_major(
            reqwest::Method::DELETE,
            "/channels/{channel_id}/messages/{message_id}/reactions/{emoji}/@me",
            format!(
                "/channels/{}/messages/{}/reactions/{}/@me",
                self.channel_id,
                self.message_id,
                self.emoji.url_part()
            ),
            self.channel_id.get(),
        );
        self.http.empty(route, None).await
    }
}

into_future!(DeleteOwnReaction<'a> => Result<()>);

/// Builder for removing a specific user's reaction
/// (`DELETE /channels/{id}/messages/{id}/reactions/{emoji}/{user_id}`).
#[derive(Debug)]
pub struct DeleteUserReaction<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    message_id: MessageId,
    emoji: Emoji,
    user_id: UserId,
}

impl<'a> DeleteUserReaction<'a> {
    pub(crate) fn new(
        http: &'a Http,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: Emoji,
        user_id: UserId,
    ) -> Self {
        Self {
            http,
            channel_id,
            message_id,
            emoji,
            user_id,
        }
    }

    pub(crate) async fn execute(self) -> Result<()> {
        let route = Route::with_major(
            reqwest::Method::DELETE,
            "/channels/{channel_id}/messages/{message_id}/reactions/{emoji}/{user_id}",
            format!(
                "/channels/{}/messages/{}/reactions/{}/{}",
                self.channel_id,
                self.message_id,
                self.emoji.url_part(),
                self.user_id
            ),
            self.channel_id.get(),
        );
        self.http.empty(route, None).await
    }
}

into_future!(DeleteUserReaction<'a> => Result<()>);

/// Builder for removing all reactions on a message
/// (`DELETE /channels/{id}/messages/{id}/reactions`).
#[derive(Debug)]
pub struct ClearReactions<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    message_id: MessageId,
}

impl<'a> ClearReactions<'a> {
    pub(crate) fn new(http: &'a Http, channel_id: ChannelId, message_id: MessageId) -> Self {
        Self {
            http,
            channel_id,
            message_id,
        }
    }

    pub(crate) async fn execute(self) -> Result<()> {
        let route = Route::with_major(
            reqwest::Method::DELETE,
            "/channels/{channel_id}/messages/{message_id}/reactions",
            format!(
                "/channels/{}/messages/{}/reactions",
                self.channel_id, self.message_id
            ),
            self.channel_id.get(),
        );
        self.http.empty(route, None).await
    }
}

into_future!(ClearReactions<'a> => Result<()>);
