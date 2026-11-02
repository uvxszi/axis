//! Community endpoints: threads and forum posts, pins, member listing and
//! search, users and DMs, the audit log, and reaction lists.

use serde::Serialize;
use serde_json::json;

use axis_core::{
    ChannelId, GenericId, GuildId, MessageId, UserId,
    error::{AxisError, Result},
};
use axis_model::{
    ActiveThreads, AllowedMentions, AuditLog, Channel, ChannelType, Embed, Emoji, Invite, Member,
    PinnedMessages, User,
};

use crate::{client::Http, routing::Route};

/// Defines a builder for an endpoint without optional parameters: the
/// struct, its constructor, its `execute`, and `IntoFuture`.
macro_rules! plain_endpoint {
    (
        $(#[$meta:meta])*
        $name:ident { $($field:ident : $ty:ty),* $(,)? } => $output:ty,
        |$this:ident| $body:block
    ) => {
        $(#[$meta])*
        #[derive(Debug)]
        pub struct $name<'a> {
            http: &'a Http,
            $($field: $ty,)*
        }

        impl<'a> $name<'a> {
            pub(crate) fn new(http: &'a Http, $($field: $ty),*) -> Self {
                Self { http, $($field),* }
            }

            pub(crate) async fn execute(self) -> Result<$output> {
                let $this = self;
                $body
            }
        }

        into_future!($name<'a> => Result<$output>);
    };
}

fn to_body<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|error| AxisError::Serialization(error.into()))
}

fn validate_thread_name(name: &str) -> Result<()> {
    let chars = name.chars().count();
    if (1..=100).contains(&chars) {
        Ok(())
    } else {
        Err(AxisError::invalid_data(format!(
            "thread name must be 1-100 characters, got {chars}"
        )))
    }
}

fn validate_archive_duration(minutes: Option<u32>) -> Result<()> {
    match minutes {
        None | Some(60 | 1440 | 4320 | 10080) => Ok(()),
        Some(other) => Err(AxisError::invalid_data(format!(
            "auto_archive_duration must be 60, 1440, 4320, or 10080 minutes, got {other}"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Threads and forum posts
// ---------------------------------------------------------------------------

/// Builder for starting a thread from an existing message
/// (`POST /channels/{id}/messages/{id}/threads`).
#[derive(Debug)]
pub struct StartThreadFromMessage<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    message_id: MessageId,
    fields: ThreadFields,
}

#[derive(Debug, Default, Serialize)]
struct ThreadFields {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_archive_duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_limit_per_user: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<ChannelType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    invitable: Option<bool>,
}

impl<'a> StartThreadFromMessage<'a> {
    pub(crate) fn new(
        http: &'a Http,
        channel_id: ChannelId,
        message_id: MessageId,
        name: String,
    ) -> Self {
        Self {
            http,
            channel_id,
            message_id,
            fields: ThreadFields {
                name,
                ..ThreadFields::default()
            },
        }
    }

    /// Minutes of inactivity before auto-archiving (60, 1440, 4320, 10080).
    pub fn auto_archive_duration(mut self, minutes: u32) -> Self {
        self.fields.auto_archive_duration = Some(minutes);
        self
    }

    /// Slowmode seconds for the thread.
    pub fn rate_limit_per_user(mut self, seconds: u32) -> Self {
        self.fields.rate_limit_per_user = Some(seconds);
        self
    }

    pub(crate) async fn execute(self) -> Result<Channel> {
        validate_thread_name(&self.fields.name)?;
        validate_archive_duration(self.fields.auto_archive_duration)?;
        let route = Route::with_major(
            reqwest::Method::POST,
            "/channels/{channel_id}/messages/{message_id}/threads",
            format!(
                "/channels/{}/messages/{}/threads",
                self.channel_id, self.message_id
            ),
            self.channel_id.get(),
        );
        self.http
            .json(route, Some(to_body(&self.fields)?), None)
            .await
    }
}

into_future!(StartThreadFromMessage<'a> => Result<Channel>);

/// Builder for starting a thread without a starter message
/// (`POST /channels/{id}/threads`). Defaults to a public thread.
#[derive(Debug)]
pub struct StartThread<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    fields: ThreadFields,
}

impl<'a> StartThread<'a> {
    pub(crate) fn new(http: &'a Http, channel_id: ChannelId, name: String) -> Self {
        Self {
            http,
            channel_id,
            fields: ThreadFields {
                name,
                r#type: Some(ChannelType::PublicThread),
                ..ThreadFields::default()
            },
        }
    }

    /// Make the thread private (only invited members and moderators see it).
    pub fn private(mut self, private: bool) -> Self {
        self.fields.r#type = Some(if private {
            ChannelType::PrivateThread
        } else {
            ChannelType::PublicThread
        });
        self
    }

    /// For private threads: whether non-moderators may add members.
    pub fn invitable(mut self, invitable: bool) -> Self {
        self.fields.invitable = Some(invitable);
        self
    }

    /// Minutes of inactivity before auto-archiving (60, 1440, 4320, 10080).
    pub fn auto_archive_duration(mut self, minutes: u32) -> Self {
        self.fields.auto_archive_duration = Some(minutes);
        self
    }

    pub(crate) async fn execute(self) -> Result<Channel> {
        validate_thread_name(&self.fields.name)?;
        validate_archive_duration(self.fields.auto_archive_duration)?;
        let route = Route::with_major(
            reqwest::Method::POST,
            "/channels/{channel_id}/threads",
            format!("/channels/{}/threads", self.channel_id),
            self.channel_id.get(),
        );
        self.http
            .json(route, Some(to_body(&self.fields)?), None)
            .await
    }
}

into_future!(StartThread<'a> => Result<Channel>);

/// Builder for creating a post in a forum or media channel
/// (`POST /channels/{id}/threads` with a starter message).
#[derive(Debug)]
pub struct CreateForumPost<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    name: String,
    content: Option<String>,
    embeds: Vec<Embed>,
    applied_tags: Vec<GenericId>,
    auto_archive_duration: Option<u32>,
    allowed_mentions: Option<AllowedMentions>,
}

impl<'a> CreateForumPost<'a> {
    pub(crate) fn new(http: &'a Http, channel_id: ChannelId, name: String) -> Self {
        Self {
            http,
            channel_id,
            name,
            content: None,
            embeds: Vec::new(),
            applied_tags: Vec::new(),
            auto_archive_duration: None,
            allowed_mentions: None,
        }
    }

    /// The starter message's content.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Embeds on the starter message.
    pub fn embeds(mut self, embeds: impl IntoIterator<Item = Embed>) -> Self {
        self.embeds = embeds.into_iter().collect();
        self
    }

    /// Tags to apply to the post (IDs from the forum's `available_tags`).
    pub fn applied_tags(mut self, tags: impl IntoIterator<Item = GenericId>) -> Self {
        self.applied_tags = tags.into_iter().collect();
        self
    }

    /// Minutes of inactivity before auto-archiving (60, 1440, 4320, 10080).
    pub fn auto_archive_duration(mut self, minutes: u32) -> Self {
        self.auto_archive_duration = Some(minutes);
        self
    }

    /// Restrict which mentions in the starter message ping.
    pub fn allowed_mentions(mut self, allowed: AllowedMentions) -> Self {
        self.allowed_mentions = Some(allowed);
        self
    }

    pub(crate) async fn execute(self) -> Result<Channel> {
        validate_thread_name(&self.name)?;
        validate_archive_duration(self.auto_archive_duration)?;
        let content_chars = self.content.as_ref().map_or(0, |c| c.chars().count());
        if content_chars > 2000 {
            return Err(AxisError::invalid_data(format!(
                "forum post content is {content_chars} characters; Discord allows at most 2000"
            )));
        }
        if content_chars == 0 && self.embeds.is_empty() {
            return Err(AxisError::invalid_data(
                "a forum post needs content or at least one embed",
            ));
        }
        let mut message = json!({});
        if let Some(content) = &self.content {
            message["content"] = json!(content);
        }
        if !self.embeds.is_empty() {
            message["embeds"] = json!(self.embeds);
        }
        if let Some(allowed) = &self.allowed_mentions {
            message["allowed_mentions"] = json!(allowed);
        }
        let mut body = json!({ "name": self.name, "message": message });
        if !self.applied_tags.is_empty() {
            body["applied_tags"] = json!(self.applied_tags);
        }
        if let Some(minutes) = self.auto_archive_duration {
            body["auto_archive_duration"] = json!(minutes);
        }
        let route = Route::with_major(
            reqwest::Method::POST,
            "/channels/{channel_id}/threads",
            format!("/channels/{}/threads", self.channel_id),
            self.channel_id.get(),
        );
        self.http.json(route, Some(to_body(&body)?), None).await
    }
}

into_future!(CreateForumPost<'a> => Result<Channel>);

plain_endpoint! {
    /// Builder for joining a thread as the bot
    /// (`PUT /channels/{id}/thread-members/@me`).
    JoinThread { thread_id: ChannelId } => (),
    |this| {
        let route = Route::with_major(
            reqwest::Method::PUT,
            "/channels/{channel_id}/thread-members/@me",
            format!("/channels/{}/thread-members/@me", this.thread_id),
            this.thread_id.get(),
        );
        this.http.empty(route, None).await
    }
}

plain_endpoint! {
    /// Builder for adding a member to a thread
    /// (`PUT /channels/{id}/thread-members/{user_id}`).
    AddThreadMember { thread_id: ChannelId, user_id: UserId } => (),
    |this| {
        let route = Route::with_major(
            reqwest::Method::PUT,
            "/channels/{channel_id}/thread-members/{user_id}",
            format!("/channels/{}/thread-members/{}", this.thread_id, this.user_id),
            this.thread_id.get(),
        );
        this.http.empty(route, None).await
    }
}

plain_endpoint! {
    /// Builder for removing a member from a thread
    /// (`DELETE /channels/{id}/thread-members/{user_id}`).
    RemoveThreadMember { thread_id: ChannelId, user_id: UserId } => (),
    |this| {
        let route = Route::with_major(
            reqwest::Method::DELETE,
            "/channels/{channel_id}/thread-members/{user_id}",
            format!("/channels/{}/thread-members/{}", this.thread_id, this.user_id),
            this.thread_id.get(),
        );
        this.http.empty(route, None).await
    }
}

plain_endpoint! {
    /// Builder for listing a guild's active threads
    /// (`GET /guilds/{id}/threads/active`).
    GetActiveThreads { guild_id: GuildId } => ActiveThreads,
    |this| {
        let route = Route::with_major(
            reqwest::Method::GET,
            "/guilds/{guild_id}/threads/active",
            format!("/guilds/{}/threads/active", this.guild_id),
            this.guild_id.get(),
        );
        this.http.json(route, None, None).await
    }
}

// ---------------------------------------------------------------------------
// Pins
// ---------------------------------------------------------------------------

plain_endpoint! {
    /// Builder for pinning a message
    /// (`PUT /channels/{id}/messages/pins/{message_id}`).
    PinMessage { channel_id: ChannelId, message_id: MessageId } => (),
    |this| {
        let route = Route::with_major(
            reqwest::Method::PUT,
            "/channels/{channel_id}/messages/pins/{message_id}",
            format!("/channels/{}/messages/pins/{}", this.channel_id, this.message_id),
            this.channel_id.get(),
        );
        this.http.empty(route, None).await
    }
}

plain_endpoint! {
    /// Builder for unpinning a message
    /// (`DELETE /channels/{id}/messages/pins/{message_id}`).
    UnpinMessage { channel_id: ChannelId, message_id: MessageId } => (),
    |this| {
        let route = Route::with_major(
            reqwest::Method::DELETE,
            "/channels/{channel_id}/messages/pins/{message_id}",
            format!("/channels/{}/messages/pins/{}", this.channel_id, this.message_id),
            this.channel_id.get(),
        );
        this.http.empty(route, None).await
    }
}

plain_endpoint! {
    /// Builder for listing a channel's pinned messages
    /// (`GET /channels/{id}/messages/pins`, first page of up to 50).
    GetPinnedMessages { channel_id: ChannelId } => PinnedMessages,
    |this| {
        let route = Route::with_major(
            reqwest::Method::GET,
            "/channels/{channel_id}/messages/pins",
            format!("/channels/{}/messages/pins", this.channel_id),
            this.channel_id.get(),
        );
        this.http.json(route, None, None).await
    }
}

// ---------------------------------------------------------------------------
// Members
// ---------------------------------------------------------------------------

/// Builder for listing guild members
/// (`GET /guilds/{id}/members`; needs the `GUILD_MEMBERS` intent).
#[derive(Debug)]
pub struct ListMembers<'a> {
    http: &'a Http,
    guild_id: GuildId,
    limit: u16,
    after: Option<UserId>,
}

impl<'a> ListMembers<'a> {
    pub(crate) fn new(http: &'a Http, guild_id: GuildId) -> Self {
        Self {
            http,
            guild_id,
            limit: 100,
            after: None,
        }
    }

    /// How many members to return (1–1000, default 100).
    pub fn limit(mut self, limit: u16) -> Self {
        self.limit = limit;
        self
    }

    /// Return members with user IDs after this one (pagination).
    pub fn after(mut self, user_id: UserId) -> Self {
        self.after = Some(user_id);
        self
    }

    pub(crate) async fn execute(self) -> Result<Vec<Member>> {
        if !(1..=1000).contains(&self.limit) {
            return Err(AxisError::invalid_data("member list limit must be 1-1000"));
        }
        let mut query = vec![("limit", self.limit.to_string())];
        if let Some(after) = self.after {
            query.push(("after", after.to_string()));
        }
        let route = Route::with_major(
            reqwest::Method::GET,
            "/guilds/{guild_id}/members",
            format!("/guilds/{}/members", self.guild_id),
            self.guild_id.get(),
        );
        self.http.json(route, None, Some(&query[..])).await
    }
}

into_future!(ListMembers<'a> => Result<Vec<Member>>);

/// Builder for searching guild members by username/nickname prefix
/// (`GET /guilds/{id}/members/search`).
#[derive(Debug)]
pub struct SearchMembers<'a> {
    http: &'a Http,
    guild_id: GuildId,
    query: String,
    limit: u16,
}

impl<'a> SearchMembers<'a> {
    pub(crate) fn new(http: &'a Http, guild_id: GuildId, query: String) -> Self {
        Self {
            http,
            guild_id,
            query,
            limit: 10,
        }
    }

    /// How many members to return (1–1000, default 10).
    pub fn limit(mut self, limit: u16) -> Self {
        self.limit = limit;
        self
    }

    pub(crate) async fn execute(self) -> Result<Vec<Member>> {
        if self.query.trim().is_empty() {
            return Err(AxisError::invalid_data("member search query is empty"));
        }
        if !(1..=1000).contains(&self.limit) {
            return Err(AxisError::invalid_data(
                "member search limit must be 1-1000",
            ));
        }
        let query = vec![
            ("query", self.query.clone()),
            ("limit", self.limit.to_string()),
        ];
        let route = Route::with_major(
            reqwest::Method::GET,
            "/guilds/{guild_id}/members/search",
            format!("/guilds/{}/members/search", self.guild_id),
            self.guild_id.get(),
        );
        self.http.json(route, None, Some(&query[..])).await
    }
}

into_future!(SearchMembers<'a> => Result<Vec<Member>>);

/// Builder for editing the bot's own guild member
/// (`PATCH /guilds/{id}/members/@me`) — e.g. its per-guild nickname.
#[derive(Debug)]
pub struct EditCurrentMember<'a> {
    http: &'a Http,
    guild_id: GuildId,
    nick: Option<Option<String>>,
}

impl<'a> EditCurrentMember<'a> {
    pub(crate) fn new(http: &'a Http, guild_id: GuildId) -> Self {
        Self {
            http,
            guild_id,
            nick: None,
        }
    }

    /// Set the bot's nickname in this guild.
    pub fn nick(mut self, nick: impl Into<String>) -> Self {
        self.nick = Some(Some(nick.into()));
        self
    }

    /// Remove the bot's nickname in this guild.
    pub fn clear_nick(mut self) -> Self {
        self.nick = Some(None);
        self
    }

    pub(crate) async fn execute(self) -> Result<Member> {
        let body = match &self.nick {
            Some(nick) => json!({ "nick": nick }),
            None => json!({}),
        };
        let route = Route::with_major(
            reqwest::Method::PATCH,
            "/guilds/{guild_id}/members/@me",
            format!("/guilds/{}/members/@me", self.guild_id),
            self.guild_id.get(),
        );
        self.http.json(route, Some(to_body(&body)?), None).await
    }
}

into_future!(EditCurrentMember<'a> => Result<Member>);

// ---------------------------------------------------------------------------
// Users and DMs
// ---------------------------------------------------------------------------

plain_endpoint! {
    /// Builder for fetching a user (`GET /users/{id}`).
    GetUser { user_id: UserId } => User,
    |this| {
        let route = Route::new(
            reqwest::Method::GET,
            "/users/{user_id}",
            format!("/users/{}", this.user_id),
        );
        this.http.json(route, None, None).await
    }
}

plain_endpoint! {
    /// Builder for fetching the bot's own user (`GET /users/@me`).
    GetCurrentUser {} => User,
    |this| {
        let route = Route::new(reqwest::Method::GET, "/users/@me", "/users/@me".to_owned());
        this.http.json(route, None, None).await
    }
}

plain_endpoint! {
    /// Builder for opening (or fetching) a DM channel with a user
    /// (`POST /users/@me/channels`).
    CreateDm { recipient_id: UserId } => Channel,
    |this| {
        let body = to_body(&json!({ "recipient_id": this.recipient_id }))?;
        let route = Route::new(
            reqwest::Method::POST,
            "/users/@me/channels",
            "/users/@me/channels".to_owned(),
        );
        this.http.json(route, Some(body), None).await
    }
}

// ---------------------------------------------------------------------------
// Audit log and reactions
// ---------------------------------------------------------------------------

/// Builder for reading a guild's audit log
/// (`GET /guilds/{id}/audit-logs`; needs `VIEW_AUDIT_LOG`).
#[derive(Debug)]
pub struct GetAuditLog<'a> {
    http: &'a Http,
    guild_id: GuildId,
    limit: u8,
    user_id: Option<UserId>,
    action_type: Option<u16>,
}

impl<'a> GetAuditLog<'a> {
    pub(crate) fn new(http: &'a Http, guild_id: GuildId) -> Self {
        Self {
            http,
            guild_id,
            limit: 50,
            user_id: None,
            action_type: None,
        }
    }

    /// How many entries to return (1–100, default 50).
    pub fn limit(mut self, limit: u8) -> Self {
        self.limit = limit;
        self
    }

    /// Only entries made by this user.
    pub fn user_id(mut self, user_id: UserId) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Only entries of this documented action type value.
    pub fn action_type(mut self, action_type: u16) -> Self {
        self.action_type = Some(action_type);
        self
    }

    pub(crate) async fn execute(self) -> Result<AuditLog> {
        if !(1..=100).contains(&self.limit) {
            return Err(AxisError::invalid_data("audit log limit must be 1-100"));
        }
        let mut query = vec![("limit", self.limit.to_string())];
        if let Some(user_id) = self.user_id {
            query.push(("user_id", user_id.to_string()));
        }
        if let Some(action_type) = self.action_type {
            query.push(("action_type", action_type.to_string()));
        }
        let route = Route::with_major(
            reqwest::Method::GET,
            "/guilds/{guild_id}/audit-logs",
            format!("/guilds/{}/audit-logs", self.guild_id),
            self.guild_id.get(),
        );
        self.http.json(route, None, Some(&query[..])).await
    }
}

into_future!(GetAuditLog<'a> => Result<AuditLog>);

plain_endpoint! {
    /// Builder for listing users who reacted with an emoji
    /// (`GET /channels/{id}/messages/{id}/reactions/{emoji}`, first 100).
    GetReactions { channel_id: ChannelId, message_id: MessageId, emoji: Emoji } => Vec<User>,
    |this| {
        let route = Route::with_major(
            reqwest::Method::GET,
            "/channels/{channel_id}/messages/{message_id}/reactions/{emoji}",
            format!(
                "/channels/{}/messages/{}/reactions/{}",
                this.channel_id,
                this.message_id,
                this.emoji.url_part()
            ),
            this.channel_id.get(),
        );
        let query = [("limit", "100".to_owned())];
        this.http.json(route, None, Some(&query[..])).await
    }
}

/// Builder for creating an invite to a channel
/// (`POST /channels/{id}/invites`).
#[derive(Debug)]
pub struct CreateInvite<'a> {
    http: &'a Http,
    channel_id: ChannelId,
    max_age: u32,
    max_uses: u32,
    temporary: bool,
    reason: Option<String>,
}

impl<'a> CreateInvite<'a> {
    pub(crate) fn new(http: &'a Http, channel_id: ChannelId) -> Self {
        Self {
            http,
            channel_id,
            max_age: 86_400,
            max_uses: 0,
            temporary: false,
            reason: None,
        }
    }

    /// Seconds until it expires (0 = never, max 604800; default 1 day).
    pub fn max_age(mut self, seconds: u32) -> Self {
        self.max_age = seconds;
        self
    }

    /// How many times it can be used (0 = unlimited, max 100).
    pub fn max_uses(mut self, uses: u32) -> Self {
        self.max_uses = uses;
        self
    }

    /// Grant temporary membership (kicked on disconnect without a role).
    pub fn temporary(mut self, temporary: bool) -> Self {
        self.temporary = temporary;
        self
    }

    /// Record a reason in the guild's audit log.
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    pub(crate) async fn execute(self) -> Result<Invite> {
        if self.max_age > 604_800 || self.max_uses > 100 {
            return Err(AxisError::invalid_data(
                "invites last at most 7 days and allow at most 100 uses",
            ));
        }
        let body = to_body(&json!({
            "max_age": self.max_age,
            "max_uses": self.max_uses,
            "temporary": self.temporary,
            "unique": true,
        }))?;
        let route = Route::with_major(
            reqwest::Method::POST,
            "/channels/{channel_id}/invites",
            format!("/channels/{}/invites", self.channel_id),
            self.channel_id.get(),
        )
        .with_reason(self.reason);
        self.http.json(route, Some(body), None).await
    }
}

into_future!(CreateInvite<'a> => Result<Invite>);

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn thread_names_are_validated() {
        let http = crate::test_support::http_for_tests();
        let error = http
            .start_thread(ChannelId::new(1), String::new())
            .await
            .expect_err("empty thread name must be rejected");
        assert!(error.to_string().contains("1-100"));
    }

    #[tokio::test]
    async fn archive_durations_are_validated() {
        let http = crate::test_support::http_for_tests();
        let error = http
            .start_thread(ChannelId::new(1), "ok".to_owned())
            .auto_archive_duration(30)
            .await
            .expect_err("30 is not a valid archive duration");
        assert!(error.to_string().contains("10080"));
    }

    #[tokio::test]
    async fn forum_posts_need_content() {
        let http = crate::test_support::http_for_tests();
        let error = http
            .create_forum_post(ChannelId::new(1), "title".to_owned())
            .await
            .expect_err("empty post must be rejected");
        assert!(error.to_string().contains("content"));
    }

    #[tokio::test]
    async fn member_search_needs_query() {
        let http = crate::test_support::http_for_tests();
        let error = http
            .search_members(GuildId::new(1), "  ".to_owned())
            .await
            .expect_err("blank query must be rejected");
        assert!(error.to_string().contains("empty"));
    }
}
