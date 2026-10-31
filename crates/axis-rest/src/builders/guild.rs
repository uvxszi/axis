//! Guild endpoints.

use axis_core::{GuildId, error::Result};
use axis_model::Guild;

use crate::{client::Http, routing::Route};

/// Builder for fetching a guild (`GET /guilds/{id}`).
#[derive(Debug)]
pub struct GetGuild<'a> {
    http: &'a Http,
    guild_id: GuildId,
}

impl<'a> GetGuild<'a> {
    pub(crate) fn new(http: &'a Http, guild_id: GuildId) -> Self {
        Self { http, guild_id }
    }

    pub(crate) async fn execute(self) -> Result<Guild> {
        let route = Route::with_major(
            reqwest::Method::GET,
            "/guilds/{guild_id}",
            format!("/guilds/{}", self.guild_id),
            self.guild_id.get(),
        );
        self.http.json(route, None, None).await
    }
}

into_future!(GetGuild<'a> => Result<Guild>);
