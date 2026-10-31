//! Discord application objects.

use serde::{Deserialize, Serialize};

use axis_core::ApplicationId;

use crate::user::User;

/// A Discord application, in the shape of `GET /applications/@me` (core
/// fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Application {
    /// The application ID.
    pub id: ApplicationId,
    /// The application name.
    pub name: String,
    /// The application icon image hash, if set.
    pub icon: Option<String>,
    /// The application description.
    pub description: String,
    /// RPC origins, if configured.
    #[serde(default)]
    pub rpc_origins: Vec<String>,
    /// Whether the bot can be added by anyone.
    pub bot_public: bool,
    /// Whether adding the bot requires an OAuth2 code grant.
    pub bot_require_code_grant: bool,
    /// The terms of service URL, if set.
    pub terms_of_service_url: Option<String>,
    /// The privacy policy URL, if set.
    pub privacy_policy_url: Option<String>,
    /// The owner's user object, when included.
    pub owner: Option<User>,
    /// The hex-encoded key for API verification.
    pub verify_key: String,
    /// The application's public flags.
    pub flags: Option<u64>,
    /// Tags describing the application's functionality.
    pub tags: Option<Vec<String>>,
    /// The guild the application is bound to, if any.
    pub guild_id: Option<axis_core::GuildId>,
    /// The custom install URL, if set.
    pub custom_install_url: Option<String>,
}

/// A partial application, as delivered in the READY payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartialApplication {
    /// The application ID.
    pub id: ApplicationId,
    /// The application's public flags.
    pub flags: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_application_deserializes() {
        let app: PartialApplication =
            serde_json::from_str(r#"{ "id": "80351110224678912", "flags": 8192 }"#).unwrap();
        assert_eq!(app.id.get(), 80_351_110_224_678_912);
        assert_eq!(app.flags, Some(8192));
    }

    #[test]
    fn application_deserializes_core_fields() {
        let app: Application = serde_json::from_str(
            r#"{
                "id": "80351110224678912",
                "name": "Axis Test App",
                "icon": "abc",
                "description": "A test application",
                "rpc_origins": [],
                "bot_public": true,
                "bot_require_code_grant": false,
                "verify_key": "hex-key",
                "flags": 8192,
                "tags": ["moderation"]
            }"#,
        )
        .unwrap();
        assert_eq!(app.name, "Axis Test App");
        assert!(app.bot_public);
        assert!(!app.bot_require_code_grant);
        assert_eq!(app.tags.as_deref(), Some(&["moderation".to_owned()][..]));
    }
}
