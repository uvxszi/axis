//! Channel endpoints: fetch, create, edit, delete channels and manage
//! their permission overwrites.

use serde::Serialize;

use axis_core::{
    ChannelId, GenericId, GuildId, Permissions,
    error::{AxisError, Result},
};
use axis_model::{Channel, ChannelType, OverwriteType};

use crate::{client::Http, routing::Route};

/// Builder for fetching a channel (`GET /channels/{id}`).
#[derive(Debug)]
pub struct GetChannel<'a> {
    http: &'a Http,
    channel_id: ChannelId,
}

impl<'a> GetChannel<'a> {
    pub(crate) fn new(http: &'a Http, channel_id: ChannelId) -> Self {
        Self { http, channel_id }
    }

    pub(crate) async fn execute(self) -> Result<Channel> {
        let route = Route::with_major(
            reqwest::Method::GET,
            "/channels/{channel_id}",
            format!("/channels/{}", self.channel_id),
            self.channel_id.get(),
        );
        self.http.json(route, None, None).await
    }
}

into_future!(GetChannel<'a> => Result<Channel>);

/// Builder for creating a channel in a guild
/// (`POST /guilds/{id}/channels`).
#[derive(Debug)]
pub struct CreateGuildChannel<'a> {
    http: &'a Http,
    guild_id: GuildId,
    fields: CreateGuildChannelFields,
}

#[derive(Debug, Default, Serialize)]
struct CreateGuildChannelFields {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<ChannelType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bitrate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_limit_per_user: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nsfw: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_id: Option<ChannelId>,
}

impl<'a> CreateGuildChannel<'a> {
    pub(crate) fn new(http: &'a Http, guild_id: GuildId, name: impl Into<String>) -> Self {
        Self {
            http,
            guild_id,
            fields: CreateGuildChannelFields {
                name: name.into(),
                ..CreateGuildChannelFields::default()
            },
        }
    }

    /// Set the channel type (defaults to a guild text channel).
    pub fn kind(mut self, kind: ChannelType) -> Self {
        self.fields.r#type = Some(kind);
        self
    }

    /// Set the channel topic.
    pub fn topic(mut self, topic: impl Into<String>) -> Self {
        self.fields.topic = Some(topic.into());
        self
    }

    /// Set the voice channel bitrate (in bits/s).
    pub fn bitrate(mut self, bitrate: u32) -> Self {
        self.fields.bitrate = Some(bitrate);
        self
    }

    /// Set the voice channel user limit.
    pub fn user_limit(mut self, user_limit: u32) -> Self {
        self.fields.user_limit = Some(user_limit);
        self
    }

    /// Set slowmode seconds between messages per user (0–21600).
    pub fn rate_limit_per_user(mut self, seconds: u32) -> Self {
        self.fields.rate_limit_per_user = Some(seconds);
        self
    }

    /// Set the sorting position of the channel.
    pub fn position(mut self, position: i32) -> Self {
        self.fields.position = Some(position);
        self
    }

    /// Mark the channel as age-restricted.
    pub fn nsfw(mut self, nsfw: bool) -> Self {
        self.fields.nsfw = Some(nsfw);
        self
    }

    /// Put the channel inside a category (or, for threads, a channel).
    pub fn parent_id(mut self, parent_id: ChannelId) -> Self {
        self.fields.parent_id = Some(parent_id);
        self
    }

    pub(crate) async fn execute(self) -> Result<Channel> {
        let name_chars = self.fields.name.chars().count();
        if !(1..=100).contains(&name_chars) {
            return Err(AxisError::invalid_data(format!(
                "channel name must be 1-100 characters, got {name_chars}"
            )));
        }
        if self
            .fields
            .rate_limit_per_user
            .is_some_and(|seconds| seconds > 21_600)
        {
            return Err(AxisError::invalid_data(
                "rate_limit_per_user must be at most 21600 seconds",
            ));
        }

        let body = serde_json::to_vec(&self.fields)
            .map_err(|error| AxisError::Serialization(error.into()))?;
        let route = Route::with_major(
            reqwest::Method::POST,
            "/guilds/{guild_id}/channels",
            format!("/guilds/{}/channels", self.guild_id),
            self.guild_id.get(),
        );
        self.http.json(route, Some(body), None).await
    }
}

into_future!(CreateGuildChannel<'a> => Result<Channel>);

/// Builder for editing a channel (`PATCH /channels/{id}`).
///
/// Unset fields are left unchanged.
#[derive(Debug)]
pub struct EditChannel<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    fields: EditChannelFields,
}

#[derive(Debug, Default, Serialize)]
struct EditChannelFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bitrate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_limit_per_user: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nsfw: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_id: Option<ChannelId>,
}

impl<'a> EditChannel<'a> {
    pub(crate) fn new(http: &'a Http, channel_id: ChannelId) -> Self {
        Self {
            http,
            channel_id,
            fields: EditChannelFields::default(),
        }
    }

    /// Rename the channel.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.fields.name = Some(name.into());
        self
    }

    /// Set the channel topic.
    pub fn topic(mut self, topic: impl Into<String>) -> Self {
        self.fields.topic = Some(topic.into());
        self
    }

    /// Set the voice channel bitrate.
    pub fn bitrate(mut self, bitrate: u32) -> Self {
        self.fields.bitrate = Some(bitrate);
        self
    }

    /// Set the voice channel user limit.
    pub fn user_limit(mut self, user_limit: u32) -> Self {
        self.fields.user_limit = Some(user_limit);
        self
    }

    /// Set slowmode seconds (0–21600).
    pub fn rate_limit_per_user(mut self, seconds: u32) -> Self {
        self.fields.rate_limit_per_user = Some(seconds);
        self
    }

    /// Set the sorting position.
    pub fn position(mut self, position: i32) -> Self {
        self.fields.position = Some(position);
        self
    }

    /// Set whether the channel is age-restricted.
    pub fn nsfw(mut self, nsfw: bool) -> Self {
        self.fields.nsfw = Some(nsfw);
        self
    }

    /// Move the channel into a category.
    pub fn parent_id(mut self, parent_id: ChannelId) -> Self {
        self.fields.parent_id = Some(parent_id);
        self
    }

    pub(crate) async fn execute(self) -> Result<Channel> {
        if let Some(name) = &self.fields.name {
            let name_chars = name.chars().count();
            if !(1..=100).contains(&name_chars) {
                return Err(AxisError::invalid_data(format!(
                    "channel name must be 1-100 characters, got {name_chars}"
                )));
            }
        }
        if self
            .fields
            .rate_limit_per_user
            .is_some_and(|seconds| seconds > 21_600)
        {
            return Err(AxisError::invalid_data(
                "rate_limit_per_user must be at most 21600 seconds",
            ));
        }

        let body = serde_json::to_vec(&self.fields)
            .map_err(|error| AxisError::Serialization(error.into()))?;
        let route = Route::with_major(
            reqwest::Method::PATCH,
            "/channels/{channel_id}",
            format!("/channels/{}", self.channel_id),
            self.channel_id.get(),
        );
        self.http.json(route, Some(body), None).await
    }
}

into_future!(EditChannel<'a> => Result<Channel>);

/// Builder for deleting a channel (`DELETE /channels/{id}`).
///
/// Returns the deleted channel object (for guild channels).
#[derive(Debug)]
pub struct DeleteChannel<'a> {
    http: &'a Http,
    channel_id: ChannelId,
}

impl<'a> DeleteChannel<'a> {
    pub(crate) fn new(http: &'a Http, channel_id: ChannelId) -> Self {
        Self { http, channel_id }
    }

    pub(crate) async fn execute(self) -> Result<Channel> {
        let route = Route::with_major(
            reqwest::Method::DELETE,
            "/channels/{channel_id}",
            format!("/channels/{}", self.channel_id),
            self.channel_id.get(),
        );
        self.http.json(route, None, None).await
    }
}

into_future!(DeleteChannel<'a> => Result<Channel>);

/// Builder for editing a channel's permission overwrite
/// (`PUT /channels/{id}/permissions/{overwrite_id}`).
///
/// Both `allow` and `deny` are sent in full: Discord replaces the
/// overwrite, so pass the complete desired sets.
#[derive(Debug)]
pub struct EditChannelPermission<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    overwrite_id: GenericId,
    overwrite_type: OverwriteType,
    allow: Permissions,
    deny: Permissions,
}

#[derive(Debug, Serialize)]
struct EditChannelPermissionFields {
    r#type: OverwriteType,
    allow: Permissions,
    deny: Permissions,
}

impl<'a> EditChannelPermission<'a> {
    pub(crate) fn new(
        http: &'a Http,
        channel_id: ChannelId,
        overwrite_id: GenericId,
        overwrite_type: OverwriteType,
    ) -> Self {
        Self {
            http,
            channel_id,
            overwrite_id,
            overwrite_type,
            allow: Permissions::empty(),
            deny: Permissions::empty(),
        }
    }

    /// Set the permissions the target gains.
    pub fn allow(mut self, allow: Permissions) -> Self {
        self.allow = allow;
        self
    }

    /// Set the permissions the target loses.
    pub fn deny(mut self, deny: Permissions) -> Self {
        self.deny = deny;
        self
    }

    pub(crate) async fn execute(self) -> Result<()> {
        let body = serde_json::to_vec(&EditChannelPermissionFields {
            r#type: self.overwrite_type,
            allow: self.allow,
            deny: self.deny,
        })
        .map_err(|error| AxisError::Serialization(error.into()))?;
        let route = Route::with_major(
            reqwest::Method::PUT,
            "/channels/{channel_id}/permissions/{overwrite_id}",
            format!(
                "/channels/{}/permissions/{}",
                self.channel_id, self.overwrite_id
            ),
            self.channel_id.get(),
        );
        self.http.empty(route, Some(body)).await
    }
}

into_future!(EditChannelPermission<'a> => Result<()>);

/// Builder for deleting a channel permission overwrite
/// (`DELETE /channels/{id}/permissions/{overwrite_id}`).
#[derive(Debug)]
pub struct DeleteChannelPermission<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    overwrite_id: GenericId,
}

impl<'a> DeleteChannelPermission<'a> {
    pub(crate) fn new(http: &'a Http, channel_id: ChannelId, overwrite_id: GenericId) -> Self {
        Self {
            http,
            channel_id,
            overwrite_id,
        }
    }

    pub(crate) async fn execute(self) -> Result<()> {
        let route = Route::with_major(
            reqwest::Method::DELETE,
            "/channels/{channel_id}/permissions/{overwrite_id}",
            format!(
                "/channels/{}/permissions/{}",
                self.channel_id, self.overwrite_id
            ),
            self.channel_id.get(),
        );
        self.http.empty(route, None).await
    }
}

into_future!(DeleteChannelPermission<'a> => Result<()>);

/// Builder for showing a typing indicator in a channel
/// (`POST /channels/{id}/typing`).
///
/// The indicator lasts ~10 seconds or until a message is sent.
#[derive(Debug)]
pub struct TriggerTyping<'a> {
    http: &'a Http,
    channel_id: ChannelId,
}

impl<'a> TriggerTyping<'a> {
    pub(crate) fn new(http: &'a Http, channel_id: ChannelId) -> Self {
        Self { http, channel_id }
    }

    pub(crate) async fn execute(self) -> Result<()> {
        let route = Route::with_major(
            reqwest::Method::POST,
            "/channels/{channel_id}/typing",
            format!("/channels/{}/typing", self.channel_id),
            self.channel_id.get(),
        );
        self.http.empty(route, None).await
    }
}

into_future!(TriggerTyping<'a> => Result<()>);

/// Builder for listing all channels in a guild
/// (`GET /guilds/{guild_id}/channels`).
#[derive(Debug)]
pub struct GetGuildChannels<'a> {
    http: &'a Http,
    guild_id: GuildId,
}

impl<'a> GetGuildChannels<'a> {
    pub(crate) fn new(http: &'a Http, guild_id: GuildId) -> Self {
        Self { http, guild_id }
    }

    pub(crate) async fn execute(self) -> Result<Vec<axis_model::Channel>> {
        let route = Route::with_major(
            reqwest::Method::GET,
            "/guilds/{guild_id}/channels",
            format!("/guilds/{}/channels", self.guild_id),
            self.guild_id.get(),
        );
        self.http.json(route, None, None).await
    }
}

into_future!(GetGuildChannels<'a> => Result<Vec<axis_model::Channel>>);
