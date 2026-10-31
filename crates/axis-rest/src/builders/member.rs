//! Guild member endpoints: fetch, kick, ban, unban, and edit members
//! (including timeouts).

use serde::Serialize;

use axis_core::{
    GuildId, Timestamp, UserId,
    error::{AxisError, Result},
};
use axis_model::Member;

use crate::{client::Http, routing::Route};

/// Builder for fetching a guild member
/// (`GET /guilds/{id}/members/{user_id}`).
#[derive(Debug)]
pub struct GetMember<'a> {
    http: &'a Http,
    guild_id: GuildId,
    user_id: UserId,
}

impl<'a> GetMember<'a> {
    pub(crate) fn new(http: &'a Http, guild_id: GuildId, user_id: UserId) -> Self {
        Self {
            http,
            guild_id,
            user_id,
        }
    }

    pub(crate) async fn execute(self) -> Result<Member> {
        let route = Route::with_major(
            reqwest::Method::GET,
            "/guilds/{guild_id}/members/{user_id}",
            format!("/guilds/{}/members/{}", self.guild_id, self.user_id),
            self.guild_id.get(),
        );
        self.http.json(route, None, None).await
    }
}

into_future!(GetMember<'a> => Result<Member>);

/// Builder for kicking a member (`DELETE /guilds/{id}/members/{user_id}`).
#[derive(Debug)]
pub struct KickMember<'a> {
    http: &'a Http,
    guild_id: GuildId,
    user_id: UserId,
}

impl<'a> KickMember<'a> {
    pub(crate) fn new(http: &'a Http, guild_id: GuildId, user_id: UserId) -> Self {
        Self {
            http,
            guild_id,
            user_id,
        }
    }

    pub(crate) async fn execute(self) -> Result<()> {
        let route = Route::with_major(
            reqwest::Method::DELETE,
            "/guilds/{guild_id}/members/{user_id}",
            format!("/guilds/{}/members/{}", self.guild_id, self.user_id),
            self.guild_id.get(),
        );
        self.http.empty(route, None).await
    }
}

into_future!(KickMember<'a> => Result<()>);

/// Builder for banning a member (`PUT /guilds/{id}/bans/{user_id}`).
#[derive(Debug)]
pub struct BanMember<'a> {
    http: &'a Http,
    guild_id: GuildId,
    user_id: UserId,
    delete_message_seconds: Option<u32>,
}

#[derive(Debug, Default, Serialize)]
struct BanMemberFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    delete_message_seconds: Option<u32>,
}

impl<'a> BanMember<'a> {
    pub(crate) fn new(http: &'a Http, guild_id: GuildId, user_id: UserId) -> Self {
        Self {
            http,
            guild_id,
            user_id,
            delete_message_seconds: None,
        }
    }

    /// Also delete messages the member sent in the last `seconds` seconds
    /// (0–604800, i.e. at most 7 days).
    pub fn delete_message_seconds(mut self, seconds: u32) -> Self {
        self.delete_message_seconds = Some(seconds);
        self
    }

    pub(crate) async fn execute(self) -> Result<()> {
        if let Some(seconds) = self.delete_message_seconds {
            if seconds > 604_800 {
                return Err(AxisError::invalid_data(
                    "delete_message_seconds must be at most 604800 (7 days)",
                ));
            }
        }

        let body = if self.delete_message_seconds.is_some() {
            Some(
                serde_json::to_vec(&BanMemberFields {
                    delete_message_seconds: self.delete_message_seconds,
                })
                .map_err(|error| AxisError::Serialization(error.into()))?,
            )
        } else {
            None
        };
        let route = Route::with_major(
            reqwest::Method::PUT,
            "/guilds/{guild_id}/bans/{user_id}",
            format!("/guilds/{}/bans/{}", self.guild_id, self.user_id),
            self.guild_id.get(),
        );
        self.http.empty(route, body).await
    }
}

into_future!(BanMember<'a> => Result<()>);

/// Builder for lifting a ban (`DELETE /guilds/{id}/bans/{user_id}`).
#[derive(Debug)]
pub struct UnbanMember<'a> {
    http: &'a Http,
    guild_id: GuildId,
    user_id: UserId,
}

impl<'a> UnbanMember<'a> {
    pub(crate) fn new(http: &'a Http, guild_id: GuildId, user_id: UserId) -> Self {
        Self {
            http,
            guild_id,
            user_id,
        }
    }

    pub(crate) async fn execute(self) -> Result<()> {
        let route = Route::with_major(
            reqwest::Method::DELETE,
            "/guilds/{guild_id}/bans/{user_id}",
            format!("/guilds/{}/bans/{}", self.guild_id, self.user_id),
            self.guild_id.get(),
        );
        self.http.empty(route, None).await
    }
}

into_future!(UnbanMember<'a> => Result<()>);

/// Builder for editing a guild member
/// (`PATCH /guilds/{id}/members/{user_id}`).
///
/// Unset fields are left unchanged. Timeouts use
/// [`communication_disabled_until`](Self::timeout_until) with a future
/// [`Timestamp`] (max 28 days ahead) and are lifted with
/// [`clear_timeout`](Self::clear_timeout) (sends `null`).
#[derive(Debug)]
pub struct EditMember<'a> {
    http: &'a Http,
    guild_id: GuildId,
    user_id: UserId,
    fields: EditMemberFields,
}

#[derive(Debug, Default, Serialize)]
struct EditMemberFields {
    /// `Some(None)` serializes as `null` (clear the nickname).
    #[serde(skip_serializing_if = "Option::is_none")]
    nick: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    roles: Option<Vec<axis_core::RoleId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deaf: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mute: Option<bool>,
    #[serde(rename = "communication_disabled_until")]
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout_until: Option<Option<Timestamp>>,
}

impl<'a> EditMember<'a> {
    pub(crate) fn new(http: &'a Http, guild_id: GuildId, user_id: UserId) -> Self {
        Self {
            http,
            guild_id,
            user_id,
            fields: EditMemberFields::default(),
        }
    }

    /// Set the member's nickname.
    pub fn nick(mut self, nick: impl Into<String>) -> Self {
        self.fields.nick = Some(Some(nick.into()));
        self
    }

    /// Remove the member's nickname (sends `null`).
    pub fn clear_nick(mut self) -> Self {
        self.fields.nick = Some(None);
        self
    }

    /// Replace the member's roles with this exact set.
    pub fn roles(mut self, roles: impl IntoIterator<Item = axis_core::RoleId>) -> Self {
        self.fields.roles = Some(roles.into_iter().collect());
        self
    }

    /// Server-deafen the member in voice channels.
    pub fn deaf(mut self, deaf: bool) -> Self {
        self.fields.deaf = Some(deaf);
        self
    }

    /// Server-mute the member in voice channels.
    pub fn mute(mut self, mute: bool) -> Self {
        self.fields.mute = Some(mute);
        self
    }

    /// Time the member out until the given timestamp (at most 28 days in
    /// the future per Discord's rules).
    pub fn timeout_until(mut self, until: Timestamp) -> Self {
        self.fields.timeout_until = Some(Some(until));
        self
    }

    /// Lift the member's timeout (sends `null`).
    pub fn clear_timeout(mut self) -> Self {
        self.fields.timeout_until = Some(None);
        self
    }

    pub(crate) async fn execute(self) -> Result<Member> {
        if let Some(Some(until)) = self.fields.timeout_until {
            let now = Timestamp::now_utc();
            if until < now {
                return Err(AxisError::invalid_data(
                    "timeout_until must be in the future (use `clear_timeout` to lift a timeout)",
                ));
            }
        }

        let body = serde_json::to_vec(&self.fields)
            .map_err(|error| AxisError::Serialization(error.into()))?;
        let route = Route::with_major(
            reqwest::Method::PATCH,
            "/guilds/{guild_id}/members/{user_id}",
            format!("/guilds/{}/members/{}", self.guild_id, self.user_id),
            self.guild_id.get(),
        );
        self.http.json(route, Some(body), None).await
    }
}

into_future!(EditMember<'a> => Result<Member>);
