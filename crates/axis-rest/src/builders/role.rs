//! Guild role endpoints: create, edit, and delete roles.

use serde::Serialize;

use axis_core::{
    GuildId, Permissions, RoleId,
    error::{AxisError, Result},
};
use axis_model::Role;

use crate::{client::Http, routing::Route};

/// Shared role fields between create and edit bodies.
#[derive(Debug, Default, Serialize)]
struct RoleFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Serialized as a decimal string, Discord's form for permission
    /// bitsets.
    #[serde(skip_serializing_if = "Option::is_none")]
    permissions: Option<Permissions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hoist: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mentionable: Option<bool>,
}

/// Builder for creating a guild role (`POST /guilds/{id}/roles`).
#[derive(Debug)]
pub struct CreateRole<'a> {
    http: &'a Http,
    guild_id: GuildId,
    fields: RoleFields,
}

impl<'a> CreateRole<'a> {
    pub(crate) fn new(http: &'a Http, guild_id: GuildId) -> Self {
        Self {
            http,
            guild_id,
            fields: RoleFields::default(),
        }
    }

    /// Set the role name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.fields.name = Some(name.into());
        self
    }

    /// Set the permissions the role grants.
    pub fn permissions(mut self, permissions: Permissions) -> Self {
        self.fields.permissions = Some(permissions);
        self
    }

    /// Set the role color as a `0xRRGGBB` integer.
    pub fn color(mut self, color: u32) -> Self {
        self.fields.color = Some(color);
        self
    }

    /// Pin the role in the member list.
    pub fn hoist(mut self, hoist: bool) -> Self {
        self.fields.hoist = Some(hoist);
        self
    }

    /// Allow users to mention the role.
    pub fn mentionable(mut self, mentionable: bool) -> Self {
        self.fields.mentionable = Some(mentionable);
        self
    }

    pub(crate) async fn execute(self) -> Result<Role> {
        if let Some(name) = &self.fields.name {
            let name_chars = name.chars().count();
            if !(1..=100).contains(&name_chars) {
                return Err(AxisError::invalid_data(format!(
                    "role name must be 1-100 characters, got {name_chars}"
                )));
            }
        }

        let body = serde_json::to_vec(&self.fields)
            .map_err(|error| AxisError::Serialization(error.into()))?;
        let route = Route::with_major(
            reqwest::Method::POST,
            "/guilds/{guild_id}/roles",
            format!("/guilds/{}/roles", self.guild_id),
            self.guild_id.get(),
        );
        self.http.json(route, Some(body), None).await
    }
}

into_future!(CreateRole<'a> => Result<Role>);

/// Builder for editing a role (`PATCH /guilds/{id}/roles/{role_id}`).
///
/// Unset fields are left unchanged.
#[derive(Debug)]
pub struct EditRole<'a> {
    http: &'a Http,
    guild_id: GuildId,
    role_id: RoleId,
    fields: RoleFields,
}

impl<'a> EditRole<'a> {
    pub(crate) fn new(http: &'a Http, guild_id: GuildId, role_id: RoleId) -> Self {
        Self {
            http,
            guild_id,
            role_id,
            fields: RoleFields::default(),
        }
    }

    /// Rename the role.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.fields.name = Some(name.into());
        self
    }

    /// Set the permissions the role grants.
    pub fn permissions(mut self, permissions: Permissions) -> Self {
        self.fields.permissions = Some(permissions);
        self
    }

    /// Set the role color.
    pub fn color(mut self, color: u32) -> Self {
        self.fields.color = Some(color);
        self
    }

    /// Pin or unpin the role in the member list.
    pub fn hoist(mut self, hoist: bool) -> Self {
        self.fields.hoist = Some(hoist);
        self
    }

    /// Allow or disallow mentioning the role.
    pub fn mentionable(mut self, mentionable: bool) -> Self {
        self.fields.mentionable = Some(mentionable);
        self
    }

    pub(crate) async fn execute(self) -> Result<Role> {
        if let Some(name) = &self.fields.name {
            let name_chars = name.chars().count();
            if !(1..=100).contains(&name_chars) {
                return Err(AxisError::invalid_data(format!(
                    "role name must be 1-100 characters, got {name_chars}"
                )));
            }
        }

        let body = serde_json::to_vec(&self.fields)
            .map_err(|error| AxisError::Serialization(error.into()))?;
        let route = Route::with_major(
            reqwest::Method::PATCH,
            "/guilds/{guild_id}/roles/{role_id}",
            format!("/guilds/{}/roles/{}", self.guild_id, self.role_id),
            self.guild_id.get(),
        );
        self.http.json(route, Some(body), None).await
    }
}

into_future!(EditRole<'a> => Result<Role>);

/// Builder for deleting a role (`DELETE /guilds/{id}/roles/{role_id}`).
#[derive(Debug)]
pub struct DeleteRole<'a> {
    http: &'a Http,
    guild_id: GuildId,
    role_id: RoleId,
}

impl<'a> DeleteRole<'a> {
    pub(crate) fn new(http: &'a Http, guild_id: GuildId, role_id: RoleId) -> Self {
        Self {
            http,
            guild_id,
            role_id,
        }
    }

    pub(crate) async fn execute(self) -> Result<()> {
        let route = Route::with_major(
            reqwest::Method::DELETE,
            "/guilds/{guild_id}/roles/{role_id}",
            format!("/guilds/{}/roles/{}", self.guild_id, self.role_id),
            self.guild_id.get(),
        );
        self.http.empty(route, None).await
    }
}

into_future!(DeleteRole<'a> => Result<()>);
