//! The Discord emoji object.

use axis_core::EmojiId;

/// An emoji, in the "partial emoji" shape used inside reactions and in the
/// full shape used in guild emoji lists.
///
/// Guild emojis carry an `id`; unicode emojis only have a `name`. The
/// `name` may be `None` for emojis whose underlying data was deleted.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Emoji {
    /// The emoji ID, if this is a custom (guild) emoji.
    pub id: Option<EmojiId>,
    /// The emoji name; for custom emojis the name without colons, for
    /// unicode emojis the emoji itself.
    pub name: Option<String>,
    /// Whether this emoji is animated.
    pub animated: Option<bool>,
}

impl Emoji {
    /// The path segment identifying this emoji for reaction endpoints,
    /// URL-encoded as Discord requires (e.g. `%F0%9F%98%80` for 👍 or
    /// `myemoji%3A1234567890` for a custom emoji).
    pub fn url_part(&self) -> String {
        let name = self.name.as_deref().unwrap_or_default();
        match self.id {
            Some(id) => percent_encode(&format!("{name}:{id}")),
            None => percent_encode(name),
        }
    }
}

/// Percent-encode a string for use in a URL path segment: every byte that
/// is not an unreserved URI character becomes `%XX`. Deliberately
/// dependency-free; emoji names and snowflakes are the only inputs.
pub(crate) fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*byte as char)
            }
            other => {
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                out.push('%');
                out.push(HEX[(*other >> 4) as usize] as char);
                out.push(HEX[(*other & 0x0F) as usize] as char);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_emoji_url_part() {
        let emoji = Emoji {
            id: None,
            name: Some("👍".to_owned()),
            animated: None,
        };
        assert_eq!(emoji.url_part(), "%F0%9F%91%8D");
    }

    #[test]
    fn custom_emoji_url_part() {
        let emoji = Emoji {
            id: Some(EmojiId::new(417_719_834_621_489_152)),
            name: Some("超大なダルマ".to_owned()),
            animated: Some(true),
        };
        // Multi-byte name, ASCII colon, ASCII digits: only non-unreserved
        // bytes are encoded (colon included).
        assert_eq!(
            emoji.url_part(),
            "%E8%B6%85%E5%A4%A7%E3%81%AA%E3%83%80%E3%83%AB%E3%83%9E%3A417719834621489152"
        );
    }

    #[test]
    fn percent_encode_keeps_unreserved() {
        assert_eq!(percent_encode("abcXYZ019-._~"), "abcXYZ019-._~");
        assert_eq!(percent_encode("a b"), "a%20b");
        assert_eq!(percent_encode(":"), "%3A");
        assert_eq!(percent_encode(""), "");
    }

    #[test]
    fn emoji_deserializes_partial_reaction_shape() {
        let emoji: Emoji =
            serde_json::from_str(r#"{ "id": null, "name": "🔥", "animated": false }"#).unwrap();
        assert_eq!(emoji.id, None);
        assert_eq!(emoji.name.as_deref(), Some("🔥"));
    }

    #[test]
    fn emoji_deserializes_custom_shape() {
        let emoji: Emoji = serde_json::from_str(
            r#"{ "id": "417719834621489152", "name": "ultra_daruma", "animated": true }"#,
        )
        .unwrap();
        assert_eq!(emoji.id.map(|id| id.get()), Some(417_719_834_621_489_152));
        assert_eq!(emoji.animated, Some(true));
    }
}
