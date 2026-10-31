//! The Discord role object.

use serde::{Deserialize, Serialize};

use axis_core::{Permissions, RoleId};

/// A guild role.
///
/// The `@everyone` role has the same ID as the guild it belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    /// The role ID.
    pub id: RoleId,
    /// The role name.
    pub name: String,
    /// The role's primary color, as a `0xRRGGBB` integer. Deprecated by
    /// Discord in favor of [`Self::colors`]; still present in responses.
    pub color: u32,
    /// The role's color set, including gradient colors.
    pub colors: RoleColors,
    /// Whether the role is pinned in the user listing.
    pub hoist: bool,
    /// The role icon image hash, if set.
    pub icon: Option<String>,
    /// The role's unicode emoji, if set.
    pub unicode_emoji: Option<String>,
    /// The role's position; roles with the same position are sorted by ID.
    pub position: i32,
    /// The permissions the role grants.
    pub permissions: Permissions,
    /// Whether the role is managed by an integration.
    pub managed: bool,
    /// Whether the role can be mentioned by users.
    pub mentionable: bool,
    /// The role's flags bitfield.
    pub flags: Option<u64>,
}

/// The color set of a role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleColors {
    /// The primary color (mirrors the deprecated `color` field).
    pub primary_color: u32,
    /// The secondary color for a gradient role, if set.
    pub secondary_color: Option<u32>,
    /// The tertiary color for a "holographic" role, if set.
    pub tertiary_color: Option<u32>,
}

impl Role {
    /// The mention string for this role, e.g. `<@&123>`.
    pub fn mention(&self) -> String {
        format!("<@&{}>", self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROLE: &str = r#"{
        "id": "41771983423143937",
        "name": "WE DEM BOYZZ!!!!!!",
        "color": 3447003,
        "colors": { "primary_color": 3447003, "secondary_color": null, "tertiary_color": null },
        "hoist": true,
        "icon": null,
        "unicode_emoji": null,
        "position": 1,
        "permissions": "66321471",
        "managed": false,
        "mentionable": false,
        "flags": 0
    }"#;

    #[test]
    fn role_deserializes() {
        let role: Role = serde_json::from_str(ROLE).unwrap();
        assert_eq!(role.id.get(), 41_771_983_423_143_937);
        assert_eq!(role.name, "WE DEM BOYZZ!!!!!!");
        assert_eq!(role.color, 3_447_003);
        assert_eq!(role.colors.primary_color, 3_447_003);
        assert_eq!(role.colors.secondary_color, None);
        assert!(role.hoist);
        assert_eq!(role.position, 1);
        // Permissions arrive as a decimal string ("66321471") — the exact
        // value is the documented example from Discord's role structure.
        assert!(role.permissions.contains(Permissions::SEND_MESSAGES));
        assert!(!role.managed);
    }

    #[test]
    fn role_permissions_from_number() {
        // Some endpoints use numbers instead of strings; both must work.
        let role: Role = serde_json::from_str(
            r#"{ "id": "1", "name": "r", "color": 0, "colors": { "primary_color": 0 },
                 "hoist": false, "position": 0, "permissions": 1024, "managed": false,
                 "mentionable": false }"#,
        )
        .unwrap();
        assert_eq!(role.permissions, Permissions::VIEW_CHANNEL);
    }

    #[test]
    fn role_roundtrips_and_mentions() {
        let role: Role = serde_json::from_str(ROLE).unwrap();
        let json = serde_json::to_string(&role).unwrap();
        // Permissions serialize back as a decimal string, Discord's form.
        assert!(json.contains(r#""permissions":"66321471""#));
        let reparsed: Role = serde_json::from_str(&json).unwrap();
        assert_eq!(reparsed, role);
        assert_eq!(role.mention(), "<@&41771983423143937>");
    }

    #[test]
    fn missing_optional_role_fields() {
        let minimal: Role = serde_json::from_str(
            r#"{ "id": "5", "name": "x", "color": 0, "colors": { "primary_color": 0 },
                 "hoist": false, "position": 0, "permissions": "0", "managed": false,
                 "mentionable": true }"#,
        )
        .unwrap();
        assert_eq!(minimal.icon, None);
        assert_eq!(minimal.unicode_emoji, None);
        assert_eq!(minimal.flags, None);
    }
}
