//! Application command registration endpoints: global and guild-scoped
//! commands plus per-guild command permissions.

use axis_core::{
    ApplicationCommandId, ApplicationId, GuildId,
    error::{AxisError, Result},
};
use axis_model::{
    ApplicationCommand, ApplicationCommandFields, ApplicationCommandPermission,
    GuildApplicationCommandPermissions,
};

use crate::{client::Http, routing::Route};

/// Builder for listing an application's global commands
/// (`GET /applications/{id}/commands`).
#[derive(Debug)]
pub struct GetGlobalCommands<'a> {
    http: &'a Http,
    application_id: ApplicationId,
}

impl<'a> GetGlobalCommands<'a> {
    pub(crate) fn new(http: &'a Http, application_id: ApplicationId) -> Self {
        Self {
            http,
            application_id,
        }
    }

    pub(crate) async fn execute(self) -> Result<Vec<ApplicationCommand>> {
        let route = Route::with_major(
            reqwest::Method::GET,
            "/applications/{application_id}/commands",
            format!("/applications/{}/commands", self.application_id),
            self.application_id.get(),
        );
        self.http.json(route, None, None).await
    }
}

into_future!(GetGlobalCommands<'a> => Result<Vec<ApplicationCommand>>);

/// Builder for creating a global command
/// (`POST /applications/{id}/commands`).
#[derive(Debug)]
pub struct CreateGlobalCommand<'a> {
    http: &'a Http,
    application_id: ApplicationId,
    fields: ApplicationCommandFields,
}

impl<'a> CreateGlobalCommand<'a> {
    pub(crate) fn new(
        http: &'a Http,
        application_id: ApplicationId,
        fields: ApplicationCommandFields,
    ) -> Self {
        Self {
            http,
            application_id,
            fields,
        }
    }

    pub(crate) async fn execute(self) -> Result<ApplicationCommand> {
        validate_fields(&self.fields)?;
        let route = Route::with_major(
            reqwest::Method::POST,
            "/applications/{application_id}/commands",
            format!("/applications/{}/commands", self.application_id),
            self.application_id.get(),
        );
        self.http
            .json(route, Some(to_body(&self.fields)?), None)
            .await
    }
}

into_future!(CreateGlobalCommand<'a> => Result<ApplicationCommand>);

/// Builder for bulk-overwriting all global commands
/// (`PUT /applications/{id}/commands`) — the recommended way to sync a
/// bot's command set.
#[derive(Debug)]
pub struct BulkOverwriteGlobalCommands<'a> {
    http: &'a Http,
    application_id: ApplicationId,
    commands: Vec<ApplicationCommandFields>,
}

impl<'a> BulkOverwriteGlobalCommands<'a> {
    pub(crate) fn new(
        http: &'a Http,
        application_id: ApplicationId,
        commands: Vec<ApplicationCommandFields>,
    ) -> Self {
        Self {
            http,
            application_id,
            commands,
        }
    }

    pub(crate) async fn execute(self) -> Result<Vec<ApplicationCommand>> {
        for command in &self.commands {
            validate_fields(command)?;
        }
        let route = Route::with_major(
            reqwest::Method::PUT,
            "/applications/{application_id}/commands",
            format!("/applications/{}/commands", self.application_id),
            self.application_id.get(),
        );
        self.http
            .json(route, Some(to_body(&self.commands)?), None)
            .await
    }
}

into_future!(BulkOverwriteGlobalCommands<'a> => Result<Vec<ApplicationCommand>>);

/// Builder for deleting a global command
/// (`DELETE /applications/{id}/commands/{command_id}`).
#[derive(Debug)]
pub struct DeleteGlobalCommand<'a> {
    http: &'a Http,
    application_id: ApplicationId,
    command_id: ApplicationCommandId,
}

impl<'a> DeleteGlobalCommand<'a> {
    pub(crate) fn new(
        http: &'a Http,
        application_id: ApplicationId,
        command_id: ApplicationCommandId,
    ) -> Self {
        Self {
            http,
            application_id,
            command_id,
        }
    }

    pub(crate) async fn execute(self) -> Result<()> {
        let route = Route::with_major(
            reqwest::Method::DELETE,
            "/applications/{application_id}/commands/{command_id}",
            format!(
                "/applications/{}/commands/{}",
                self.application_id, self.command_id
            ),
            self.application_id.get(),
        );
        self.http.empty(route, None).await
    }
}

into_future!(DeleteGlobalCommand<'a> => Result<()>);

/// Builder for bulk-overwriting all commands in a guild
/// (`PUT /applications/{id}/guilds/{guild_id}/commands`).
#[derive(Debug)]
pub struct BulkOverwriteGuildCommands<'a> {
    http: &'a Http,
    application_id: ApplicationId,
    guild_id: GuildId,
    commands: Vec<ApplicationCommandFields>,
}

impl<'a> BulkOverwriteGuildCommands<'a> {
    pub(crate) fn new(
        http: &'a Http,
        application_id: ApplicationId,
        guild_id: GuildId,
        commands: Vec<ApplicationCommandFields>,
    ) -> Self {
        Self {
            http,
            application_id,
            guild_id,
            commands,
        }
    }

    pub(crate) async fn execute(self) -> Result<Vec<ApplicationCommand>> {
        for command in &self.commands {
            validate_fields(command)?;
        }
        let route = Route::with_major(
            reqwest::Method::PUT,
            "/applications/{application_id}/guilds/{guild_id}/commands",
            format!(
                "/applications/{}/guilds/{}/commands",
                self.application_id, self.guild_id
            ),
            self.application_id.get(),
        );
        self.http
            .json(route, Some(to_body(&self.commands)?), None)
            .await
    }
}

into_future!(BulkOverwriteGuildCommands<'a> => Result<Vec<ApplicationCommand>>);

/// Builder for deleting a guild command
/// (`DELETE /applications/{id}/guilds/{guild_id}/commands/{command_id}`).
#[derive(Debug)]
pub struct DeleteGuildCommand<'a> {
    http: &'a Http,
    application_id: ApplicationId,
    guild_id: GuildId,
    command_id: ApplicationCommandId,
}

impl<'a> DeleteGuildCommand<'a> {
    pub(crate) fn new(
        http: &'a Http,
        application_id: ApplicationId,
        guild_id: GuildId,
        command_id: ApplicationCommandId,
    ) -> Self {
        Self {
            http,
            application_id,
            guild_id,
            command_id,
        }
    }

    pub(crate) async fn execute(self) -> Result<()> {
        let route = Route::with_major(
            reqwest::Method::DELETE,
            "/applications/{application_id}/guilds/{guild_id}/commands/{command_id}",
            format!(
                "/applications/{}/guilds/{}/commands/{}",
                self.application_id, self.guild_id, self.command_id
            ),
            self.application_id.get(),
        );
        self.http.empty(route, None).await
    }
}

into_future!(DeleteGuildCommand<'a> => Result<()>);

/// Builder for listing a guild's command permissions
/// (`GET /applications/{id}/guilds/{guild_id}/commands/permissions`).
#[derive(Debug)]
pub struct GetGuildCommandPermissions<'a> {
    http: &'a Http,
    application_id: ApplicationId,
    guild_id: GuildId,
}

impl<'a> GetGuildCommandPermissions<'a> {
    pub(crate) fn new(http: &'a Http, application_id: ApplicationId, guild_id: GuildId) -> Self {
        Self {
            http,
            application_id,
            guild_id,
        }
    }

    pub(crate) async fn execute(self) -> Result<Vec<GuildApplicationCommandPermissions>> {
        let route = Route::with_major(
            reqwest::Method::GET,
            "/applications/{application_id}/guilds/{guild_id}/commands/permissions",
            format!(
                "/applications/{}/guilds/{}/commands/permissions",
                self.application_id, self.guild_id
            ),
            self.application_id.get(),
        );
        self.http.json(route, None, None).await
    }
}

into_future!(GetGuildCommandPermissions<'a> => Result<Vec<GuildApplicationCommandPermissions>>);

/// Builder for replacing a command's per-guild permissions
/// (`PUT /applications/{id}/guilds/{guild_id}/commands/{command_id}/permissions`).
///
/// The provided entries fully replace the existing ones. Note the docs:
/// editing command permissions through a bot token is deprecated by
/// Discord in favor of the Developer Portal UI — the endpoint may refuse
/// bot-token requests.
#[derive(Debug)]
pub struct SetGuildCommandPermissions<'a> {
    http: &'a Http,
    application_id: ApplicationId,
    guild_id: GuildId,
    command_id: ApplicationCommandId,
    permissions: Vec<ApplicationCommandPermission>,
}

impl<'a> SetGuildCommandPermissions<'a> {
    pub(crate) fn new(
        http: &'a Http,
        application_id: ApplicationId,
        guild_id: GuildId,
        command_id: ApplicationCommandId,
        permissions: Vec<ApplicationCommandPermission>,
    ) -> Self {
        Self {
            http,
            application_id,
            guild_id,
            command_id,
            permissions,
        }
    }

    pub(crate) async fn execute(self) -> Result<GuildApplicationCommandPermissions> {
        if self.permissions.len() > 100 {
            return Err(AxisError::invalid_data(
                "command permissions are limited to 100 entries",
            ));
        }
        let body = serde_json::json!({ "permissions": self.permissions });
        let route = Route::with_major(
            reqwest::Method::PUT,
            "/applications/{application_id}/guilds/{guild_id}/commands/{command_id}/permissions",
            format!(
                "/applications/{}/guilds/{}/commands/{}/permissions",
                self.application_id, self.guild_id, self.command_id
            ),
            self.application_id.get(),
        );
        self.http
            .json(
                route,
                Some(serde_json::to_vec(&body).map_err(serialization)?),
                None,
            )
            .await
    }
}

into_future!(SetGuildCommandPermissions<'a> => Result<GuildApplicationCommandPermissions>);

fn validate_fields(fields: &ApplicationCommandFields) -> Result<()> {
    if let Some(name) = &fields.name {
        let chars = name.chars().count();
        if !(1..=32).contains(&chars) {
            return Err(AxisError::invalid_data(format!(
                "command name must be 1-32 characters, got {chars}"
            )));
        }
    }
    if let Some(description) = &fields.description {
        let chars = description.chars().count();
        if !(1..=100).contains(&chars) {
            return Err(AxisError::invalid_data(format!(
                "command description must be 1-100 characters, got {chars}"
            )));
        }
    }
    Ok(())
}

fn to_body<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|error| AxisError::Serialization(error.into()))
}

fn serialization(error: serde_json::Error) -> AxisError {
    AxisError::Serialization(error.into())
}
