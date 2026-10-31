//! The Discord guild member object.

use serde::{Deserialize, Serialize};

use axis_core::{Permissions, RoleId, Timestamp};

use crate::user::User;

/// A guild member: a user's presence in one specific guild.
///
/// The same member object appears in many contexts with different subsets
/// of fields populated (for example, `user` is absent in some gateway
/// events), so nearly everything is optional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    /// The underlying user, when included by the endpoint.
    pub user: Option<User>,
    /// The member's guild-specific nickname, if set.
    pub nick: Option<String>,
    /// The member's guild-specific avatar image hash, if set.
    pub avatar: Option<String>,
    /// The member's guild-specific banner image hash, if set.
    pub banner: Option<String>,
    /// The role IDs the member has.
    pub roles: Vec<RoleId>,
    /// When the member joined the guild (absent or null in some gateway
    /// contexts and for pending guests).
    pub joined_at: Option<Timestamp>,
    /// When the member started boosting the guild, if boosting.
    pub premium_since: Option<Timestamp>,
    /// Whether the member is deafened in voice channels.
    pub deaf: Option<bool>,
    /// Whether the member is muted in voice channels.
    pub mute: Option<bool>,
    /// The member flags bitfield.
    pub flags: Option<u64>,
    /// Whether the member has passed the guild's membership screening.
    pub pending: Option<bool>,
    /// Total permissions of the member in the current context (interaction
    /// payloads only).
    pub permissions: Option<Permissions>,
    /// When the member's timeout will expire, if timed out.
    pub communication_disabled_until: Option<Timestamp>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const MEMBER: &str = r#"{
        "user": { "id": "80351110224678912", "username": "Nelly", "discriminator": "0" },
        "nick": "CEO of nostalgia",
        "roles": ["41771983423143937", "290926798999268355"],
        "joined_at": "2015-04-26T06:26:56.936000+00:00",
        "premium_since": null,
        "deaf": false,
        "mute": true,
        "flags": 0,
        "pending": false,
        "communication_disabled_until": "2026-01-01T00:00:00.000000+00:00"
    }"#;

    #[test]
    fn member_deserializes() {
        let member: Member = serde_json::from_str(MEMBER).unwrap();
        let user = member.user.unwrap();
        assert_eq!(user.username, "Nelly");
        assert_eq!(member.nick.as_deref(), Some("CEO of nostalgia"));
        assert_eq!(member.roles.len(), 2);
        assert_eq!(member.roles[0].get(), 41_771_983_423_143_937);
        assert!(member.joined_at.is_some());
        assert_eq!(member.deaf, Some(false));
        assert_eq!(member.mute, Some(true));
        assert!(member.communication_disabled_until.is_some());
    }

    #[test]
    fn minimal_member() {
        // Interaction contexts provide trimmed members.
        let member: Member = serde_json::from_str(
            r#"{
                "roles": [],
                "deaf": false,
                "mute": false,
                "joined_at": "2020-01-01T00:00:00.000000+00:00"
            }"#,
        )
        .unwrap();
        assert_eq!(member.user, None);
        assert_eq!(member.pending, None);
        assert_eq!(member.flags, None);
    }

    #[test]
    fn member_roundtrips() {
        let member: Member = serde_json::from_str(MEMBER).unwrap();
        let json = serde_json::to_string(&member).unwrap();
        let reparsed: Member = serde_json::from_str(&json).unwrap();
        assert_eq!(reparsed, member);
    }
}
