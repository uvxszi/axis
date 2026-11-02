//! The Discord message object and its related types (embeds, attachments,
//! reactions, message references).

use bitflags::bitflags;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use axis_core::{ChannelId, GuildId, MessageId, Timestamp, UserId, WebhookId, id::RoleId};

use crate::{emoji::Emoji, member::Member, user::User};

bitflags! {
    /// Flags on a message.
    ///
    /// Deserialization is lenient: unknown bits (Discord may add flags)
    /// are dropped rather than rejected.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MessageFlags: u64 {
        /// The message has been crossposted to other servers.
        const CROSSPOSTED = 1 << 0;
        /// The message originated from another server (crossposted).
        const IS_CROSSPOST = 1 << 1;
        /// Embeds on the message are hidden.
        const SUPPRESS_EMBEDS = 1 << 2;
        /// The message this message is a reply/pin of was deleted.
        const SOURCE_MESSAGE_DELETED = 1 << 3;
        /// The message is urgent.
        const URGENT = 1 << 4;
        /// The message has an associated thread (with the same ID).
        const HAS_THREAD = 1 << 5;
        /// The message is only visible to the user who invoked the interaction.
        const EPHEMERAL = 1 << 6;
        /// The message is an interaction response still being loaded.
        const LOADING = 1 << 7;
        /// The message failed to mention some roles in its thread.
        const FAILED_TO_MENTION_SOME_ROLES_IN_THREAD = 1 << 8;
        /// The message does not trigger push/desktop notifications.
        const SUPPRESS_NOTIFICATIONS = 1 << 12;
        /// The message is a voice message.
        const IS_VOICE_MESSAGE = 1 << 13;
        /// The message has a message snapshot.
        const HAS_SNAPSHOT = 1 << 14;
        /// The message uses Components V2 (permanent once set).
        const IS_COMPONENTS_V2 = 1 << 15;
    }
}

impl Serialize for MessageFlags {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.bits())
    }
}

impl<'de> Deserialize<'de> for MessageFlags {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FlagsVisitor;

        impl de::Visitor<'_> for FlagsVisitor {
            type Value = MessageFlags;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("a message flags bit value (u64)")
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(MessageFlags::from_bits_truncate(value))
            }
        }

        deserializer.deserialize_u64(FlagsVisitor)
    }
}

api_enum! {
    /// The type of a message.
    ///
    /// Values from the official Message Type table. `type` is a required
    /// field, so an undocumented value fails deserialization loudly — this
    /// enum is extended as Discord adds types.
    MessageType: u8 {
        /// A normal message.
        Default = 0,
        /// A recipient was added to a group DM.
        RecipientAdd = 1,
        /// A recipient was removed from a group DM.
        RecipientRemove = 2,
        /// A group DM call.
        Call = 3,
        /// A channel name was changed.
        ChannelNameChange = 4,
        /// A channel icon was changed.
        ChannelIconChange = 5,
        /// A message was pinned.
        ChannelPinnedMessage = 6,
        /// A member joined (welcome system message).
        UserJoin = 7,
        /// The guild was boosted.
        GuildBoost = 8,
        /// The guild reached boost tier 1.
        GuildBoostTier1 = 9,
        /// The guild reached boost tier 2.
        GuildBoostTier2 = 10,
        /// The guild reached boost tier 3.
        GuildBoostTier3 = 11,
        /// The channel started following an announcement channel.
        ChannelFollowAdd = 12,
        /// The guild was disqualified from discovery.
        GuildDiscoveryDisqualified = 14,
        /// The guild was requalified for discovery.
        GuildDiscoveryRequalified = 15,
        /// Initial discovery grace period warning.
        GuildDiscoveryGracePeriodInitialWarning = 16,
        /// Final discovery grace period warning.
        GuildDiscoveryGracePeriodFinalWarning = 17,
        /// A thread was created.
        ThreadCreated = 18,
        /// A reply to another message.
        Reply = 19,
        /// A slash command invocation.
        ChatInputCommand = 20,
        /// The first message in a thread.
        ThreadStarterMessage = 21,
        /// A reminder of guild invites.
        GuildInviteReminder = 22,
        /// A context menu command invocation.
        ContextMenuCommand = 23,
        /// An auto-moderation action.
        AutoModerationAction = 24,
        /// A role subscription purchase.
        RoleSubscriptionPurchase = 25,
        /// An interaction premium upsell.
        InteractionPremiumUpsell = 26,
        /// A stage started.
        StageStart = 27,
        /// A stage ended.
        StageEnd = 28,
        /// A stage speaker change.
        StageSpeaker = 29,
        /// A stage topic change.
        StageTopic = 31,
        /// An application subscription in the guild.
        GuildApplicationPremiumSubscription = 32,
        /// Incident alert mode was enabled.
        GuildIncidentAlertModeEnabled = 36,
        /// Incident alert mode was disabled.
        GuildIncidentAlertModeDisabled = 37,
        /// A raid was reported.
        GuildIncidentReportRaid = 38,
        /// A false alarm raid report.
        GuildIncidentReportFalseAlarm = 39,
        /// A purchase notification.
        PurchaseNotification = 44,
        /// A poll result.
        PollResult = 46,
    }
}

/// A message sent in a channel.
///
/// Without the `MESSAGE_CONTENT` privileged intent, `content`, `embeds`,
/// `attachments`, and `components` come back empty for messages not
/// mentioning the bot; the fields are still present.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// The message ID.
    pub id: MessageId,
    /// The ID of the channel the message was sent in.
    pub channel_id: ChannelId,
    /// The guild the message was sent in, when applicable (absent in DMs
    /// and ephemeral messages).
    pub guild_id: Option<GuildId>,
    /// The author. May be a webhook pseudo-user; for some gateway events
    /// this is a partial user.
    pub author: User,
    /// The message content (empty when not sent or not visible).
    pub content: String,
    /// When the message was sent.
    pub timestamp: Timestamp,
    /// When the message was last edited, if ever.
    pub edited_timestamp: Option<Timestamp>,
    /// Whether the message was sent with text-to-speech.
    pub tts: bool,
    /// Whether the message mentions `@everyone` or `@here`.
    pub mention_everyone: bool,
    /// Users specifically mentioned in the message.
    pub mentions: Vec<User>,
    /// Roles specifically mentioned in the message.
    pub mention_roles: Vec<RoleId>,
    /// Files attached to the message.
    pub attachments: Vec<Attachment>,
    /// Embeds on the message.
    pub embeds: Vec<Embed>,
    /// Reactions on the message, if any.
    pub reactions: Option<Vec<Reaction>>,
    /// Whether the message is pinned.
    pub pinned: bool,
    /// The webhook that sent the message, if sent by a webhook.
    pub webhook_id: Option<WebhookId>,
    /// The message type. Types this library does not know yet are read
    /// as [`MessageType::Default`] instead of dropping the message.
    #[serde(deserialize_with = "lenient_message_type")]
    pub r#type: MessageType,
    /// The message flags.
    pub flags: Option<MessageFlags>,
    /// Reference data for replies and forwards.
    pub message_reference: Option<MessageReference>,
    /// The author's guild member data (without `user`), on gateway
    /// `MESSAGE_CREATE` in guilds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<Member>,
    /// The message this one replies to, when Discord resolved it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub referenced_message: Option<Box<Message>>,
    /// The thread started from this message, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread: Option<crate::channel::Channel>,
    /// Stickers sent with the message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sticker_items: Option<Vec<StickerItem>>,
    /// Message components (buttons, and with Components V2 the whole
    /// layout: text blocks, media galleries, containers), kept raw.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<serde_json::Value>,
}

impl Message {
    /// All readable text in the message: content, embeds, and Components
    /// V2 text blocks.
    pub fn full_text(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if !self.content.trim().is_empty() {
            parts.push(self.content.clone());
        }
        for embed in &self.embeds {
            let mut lines = Vec::new();
            if let Some(author) = &embed.author {
                lines.push(author.name.clone());
            }
            if let Some(title) = &embed.title {
                lines.push(format!("**{title}**"));
            }
            if let Some(description) = &embed.description {
                lines.push(description.clone());
            }
            for field in embed.fields.iter().flatten() {
                lines.push(format!("{}: {}", field.name, field.value));
            }
            if let Some(footer) = &embed.footer {
                lines.push(footer.text.clone());
            }
            if !lines.is_empty() {
                parts.push(format!("[embed] {}", lines.join(" | ")));
            }
        }
        let mut texts = Vec::new();
        let mut media = Vec::new();
        walk_components(&self.components, &mut texts, &mut media);
        if !texts.is_empty() {
            parts.push(texts.join("\n"));
        }
        parts.join("\n")
    }

    /// Image URLs in embeds and Components V2 media (galleries,
    /// thumbnails, files).
    pub fn media_urls(&self) -> Vec<String> {
        let mut urls = Vec::new();
        for embed in &self.embeds {
            for image in [&embed.image, &embed.thumbnail].into_iter().flatten() {
                if let Some(url) = image.proxy_url.clone().or_else(|| image.url.clone()) {
                    urls.push(url);
                }
            }
        }
        let mut texts = Vec::new();
        walk_components(&self.components, &mut texts, &mut urls);
        urls.dedup();
        urls
    }
}

/// Collect text and media URLs from (possibly nested) components.
fn walk_components(
    components: &[serde_json::Value],
    texts: &mut Vec<String>,
    media: &mut Vec<String>,
) {
    use serde_json::Value;
    for component in components {
        match component.get("type").and_then(Value::as_u64) {
            // Text Display
            Some(10) => {
                if let Some(text) = component.get("content").and_then(Value::as_str) {
                    texts.push(text.to_owned());
                }
            }
            // Thumbnail
            Some(11) => {
                if let Some(url) = component.pointer("/media/url").and_then(Value::as_str) {
                    media.push(url.to_owned());
                }
            }
            // Media Gallery
            Some(12) => {
                for item in component
                    .get("items")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    if let Some(url) = item.pointer("/media/url").and_then(Value::as_str) {
                        media.push(url.to_owned());
                    }
                }
            }
            // File
            Some(13) => {
                if let Some(url) = component.pointer("/file/url").and_then(Value::as_str) {
                    media.push(url.to_owned());
                }
            }
            _ => {}
        }
        for key in ["components", "accessory"] {
            match component.get(key) {
                Some(Value::Array(children)) => walk_components(children, texts, media),
                Some(child @ Value::Object(_)) => {
                    walk_components(std::slice::from_ref(child), texts, media)
                }
                _ => {}
            }
        }
    }
}

/// Read a message type, mapping undocumented values to
/// [`MessageType::Default`]: a new system message type must not make a
/// whole `MESSAGE_CREATE` fail to parse.
fn lenient_message_type<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<MessageType, D::Error> {
    let value = u64::deserialize(deserializer)?;
    Ok(u8::try_from(value)
        .ok()
        .and_then(MessageType::from_value)
        .unwrap_or(MessageType::Default))
}

/// The minimal sticker shape included on messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StickerItem {
    /// The sticker ID.
    pub id: axis_core::StickerId,
    /// The sticker name.
    pub name: String,
    /// 1 PNG, 2 APNG, 3 Lottie, 4 GIF.
    #[serde(default)]
    pub format_type: u8,
}

impl StickerItem {
    /// The sticker's image URL; `None` for Lottie (vector JSON) stickers.
    pub fn image_url(&self) -> Option<String> {
        match self.format_type {
            3 => None,
            4 => Some(format!("https://media.discordapp.net/stickers/{}.gif", self.id)),
            _ => Some(format!("https://media.discordapp.net/stickers/{}.png", self.id)),
        }
    }
}

/// Controls which mentions in a sent message actually ping.
///
/// The default (`AllowedMentions::none()`) pings nobody — the safe choice
/// for bots that echo user or model-generated text.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AllowedMentions {
    /// Mention kinds parsed from content: `"users"`, `"roles"`,
    /// `"everyone"`.
    pub parse: Vec<String>,
    /// Specific users that may be pinged (max 100).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub users: Vec<UserId>,
    /// Specific roles that may be pinged (max 100).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<RoleId>,
    /// Whether a reply pings the replied-to author.
    #[serde(default)]
    pub replied_user: bool,
}

impl AllowedMentions {
    /// Ping nobody.
    pub fn none() -> Self {
        Self::default()
    }

    /// Ping mentioned users (but not roles or everyone).
    pub fn users() -> Self {
        Self {
            parse: vec!["users".to_owned()],
            ..Self::default()
        }
    }

    /// Ping users and roles, but never `@everyone`/`@here`.
    pub fn users_and_roles() -> Self {
        Self {
            parse: vec!["users".to_owned(), "roles".to_owned()],
            ..Self::default()
        }
    }

    /// Also ping the author of the replied-to message.
    pub fn with_replied_user(mut self, ping: bool) -> Self {
        self.replied_user = ping;
        self
    }
}

/// A poll to attach to a new message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollCreate {
    /// The question.
    pub question: PollMedia,
    /// The answers (1–10).
    pub answers: Vec<PollAnswer>,
    /// How long the poll runs, in hours (1–768).
    pub duration: u32,
    /// Whether voters may pick several answers.
    pub allow_multiselect: bool,
}

impl PollCreate {
    /// Build a single-choice poll from a question and answer texts.
    pub fn new(
        question: impl Into<String>,
        answers: impl IntoIterator<Item = String>,
        hours: u32,
    ) -> Self {
        Self {
            question: PollMedia {
                text: question.into(),
            },
            answers: answers
                .into_iter()
                .map(|text| PollAnswer {
                    poll_media: PollMedia { text },
                })
                .collect(),
            duration: hours,
            allow_multiselect: false,
        }
    }
}

/// The text of a poll question or answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollMedia {
    /// The text (question up to 300, answers up to 55 characters).
    pub text: String,
}

/// A poll answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollAnswer {
    /// The answer text.
    pub poll_media: PollMedia,
}

/// One entry of `GET /channels/{id}/messages/pins`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessagePin {
    /// When the message was pinned.
    pub pinned_at: Timestamp,
    /// The pinned message.
    pub message: Message,
}

/// The response of `GET /channels/{id}/messages/pins`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinnedMessages {
    /// The pins, newest first.
    pub items: Vec<MessagePin>,
    /// Whether more pins exist past this page.
    #[serde(default)]
    pub has_more: bool,
}

impl Message {
    /// The mention string for the message author, e.g. `<@80351110224678912>`.
    pub fn author_mention(&self) -> String {
        format!("<@{}>", self.author.id)
    }
}

/// A reference to another message (for replies and forwards).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct MessageReference {
    /// The referenced message ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<MessageId>,
    /// The channel the referenced message is in (required for forwards).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<ChannelId>,
    /// The guild the referenced message is in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<GuildId>,
    /// Whether replying to a deleted message should error (defaults to
    /// true); set false to send the reply without a link when missing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_if_not_exists: Option<bool>,
}

/// A file attached to a message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    /// The attachment ID.
    pub id: MessageId,
    /// The file name.
    pub filename: String,
    /// The media title, if set.
    pub title: Option<String>,
    /// The alt-text description, if set (max 1024 characters).
    pub description: Option<String>,
    /// The media type, if known.
    pub content_type: Option<String>,
    /// The file size in bytes.
    pub size: u64,
    /// The source URL of the file.
    pub url: String,
    /// A proxied (CDN) URL of the file.
    pub proxy_url: Option<String>,
    /// The image/video height, if applicable.
    pub height: Option<u32>,
    /// The image/video width, if applicable.
    pub width: Option<u32>,
    /// Whether the attachment is ephemeral (e.g. inside interactions).
    pub ephemeral: Option<bool>,
}

/// A reaction on a message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reaction {
    /// Total number of times the emoji has been used (including burst
    /// reactions).
    pub count: u32,
    /// Whether the current user reacted with this emoji.
    pub me: bool,
    /// The reacted emoji, in the partial emoji shape.
    pub emoji: Emoji,
}

/// An embed attached to a message (the rich embed subset that bots can
/// send). All fields are optional except where Discord requires content.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Embed {
    /// The embed title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The embed type ("rich", "image", ...); ignored when sending.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// The embed description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The URL the title links to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The embed's timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<Timestamp>,
    /// The embed's side color, as a `0xRRGGBB` integer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    /// The footer information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<EmbedFooter>,
    /// The large image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<EmbedImage>,
    /// The small image beside the title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<EmbedThumbnail>,
    /// The video (only present in received embeds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<EmbedImage>,
    /// The provider (only present in received embeds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<EmbedProvider>,
    /// The author information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<EmbedAuthor>,
    /// The embed fields (up to 25).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<EmbedField>>,
}

/// The footer of an embed.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EmbedFooter {
    /// The footer text.
    pub text: String,
    /// The footer icon URL, if set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

/// An image in an embed.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EmbedImage {
    /// The image URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The proxied (CDN) URL of the image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
    /// The image height in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// The image width in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
}

/// The thumbnail of an embed (same shape as [`EmbedImage`]).
pub type EmbedThumbnail = EmbedImage;

/// The provider of an embed (only present in received embeds).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EmbedProvider {
    /// The provider name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The provider URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// The author block of an embed.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EmbedAuthor {
    /// The author name.
    pub name: String,
    /// The URL the author name links to, if set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The author icon URL, if set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

/// A field in an embed.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EmbedField {
    /// The field name (max 256 characters).
    pub name: String,
    /// The field value (max 1024 characters).
    pub value: String,
    /// Whether the field renders side-by-side with other inline fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline: Option<bool>,
}

/// A partial message, as delivered by gateway `MESSAGE_UPDATE`.
///
/// Message updates carry only the fields that Discord includes for the
/// update — most fields are optional, and `author` may be a partial user.
/// Treat every field as potentially absent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartialMessage {
    /// The message ID.
    pub id: MessageId,
    /// The ID of the channel the message is in.
    pub channel_id: ChannelId,
    /// The guild the message is in, when applicable.
    pub guild_id: Option<GuildId>,
    /// The author; may be a partial user object in updates.
    pub author: Option<User>,
    /// The message content, when included.
    pub content: Option<String>,
    /// When the message was sent, when included.
    pub timestamp: Option<Timestamp>,
    /// When the message was last edited, when included.
    pub edited_timestamp: Option<Timestamp>,
    /// Always `false` in message updates, per the docs.
    pub tts: Option<bool>,
    /// Whether the message mentions `@everyone`/`@here`, when included.
    pub mention_everyone: Option<bool>,
    /// Users mentioned, when included.
    pub mentions: Option<Vec<User>>,
    /// Roles mentioned, when included.
    pub mention_roles: Option<Vec<RoleId>>,
    /// Attachments, when included.
    pub attachments: Option<Vec<Attachment>>,
    /// Embeds, when included.
    pub embeds: Option<Vec<Embed>>,
    /// Whether the message is pinned, when included.
    pub pinned: Option<bool>,
    /// The message flags, when included.
    pub flags: Option<MessageFlags>,
    /// The message type, when included.
    pub r#type: Option<MessageType>,
}

/// A message deletion notification (`MESSAGE_DELETE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDelete {
    /// The deleted message's ID.
    pub id: MessageId,
    /// The channel the message was in.
    pub channel_id: ChannelId,
    /// The guild the message was in, when applicable.
    pub guild_id: Option<GuildId>,
}

/// A bulk deletion notification (`MESSAGE_DELETE_BULK`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDeleteBulk {
    /// The deleted messages' IDs.
    pub ids: Vec<MessageId>,
    /// The channel the messages were in.
    pub channel_id: ChannelId,
    /// The guild the messages were in, when applicable.
    pub guild_id: Option<GuildId>,
}

/// A reaction added to a message (`MESSAGE_REACTION_ADD`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageReactionAdd {
    /// The user who reacted.
    pub user_id: UserId,
    /// The channel containing the message.
    pub channel_id: ChannelId,
    /// The message that was reacted to.
    pub message_id: MessageId,
    /// The guild containing the channel, when applicable.
    pub guild_id: Option<GuildId>,
    /// The member who reacted, when the reaction happened in a guild.
    pub member: Option<Member>,
    /// The reacted emoji, in the partial emoji shape.
    pub emoji: Emoji,
    /// The author of the reacted message, when Discord includes it.
    pub message_author_id: Option<UserId>,
    /// Whether this is a super reaction.
    pub burst: bool,
    /// The super reaction's color palette, when present.
    pub burst_colors: Option<Vec<String>>,
    /// The documented reaction type value (0 = normal, 1 = super reaction).
    pub r#type: u8,
}

/// A reaction removed from a message (`MESSAGE_REACTION_REMOVE`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageReactionRemove {
    /// The user whose reaction was removed.
    pub user_id: UserId,
    /// The channel containing the message.
    pub channel_id: ChannelId,
    /// The message the reaction was on.
    pub message_id: MessageId,
    /// The guild containing the channel, when applicable.
    pub guild_id: Option<GuildId>,
    /// The removed emoji, in the partial emoji shape.
    pub emoji: Emoji,
    /// Whether the removed reaction was a super reaction.
    pub burst: bool,
    /// The documented reaction type value (0 = normal, 1 = super reaction).
    pub r#type: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Message payload in the standard shape, with the documented example
    /// values where available.
    const MESSAGE: &str = r#"{
        "id": "1021934727088820224",
        "channel_id": "838309323191025706",
        "author": {
            "id": "80351110224678912",
            "username": "Nelly",
            "discriminator": "0",
            "avatar": "3370dadd186e2fcd8e5d22feb5674c88"
        },
        "content": "field with inline",
        "timestamp": "2022-09-22T14:00:00.436000+00:00",
        "edited_timestamp": "2022-09-22T14:01:00.000000+00:00",
        "tts": false,
        "mention_everyone": false,
        "mentions": [
            { "id": "740485264220672102", "username": "flyric", "discriminator": "0",
              "mention": { "id": "740485264220672102", "username": "flyric", "discriminator": "0" } }
        ],
        "mention_roles": [],
        "attachments": [
            {
                "id": "1021934726123061259",
                "filename": "review.png",
                "content_type": "image/png",
                "size": 12345,
                "url": "https://cdn.discordapp.com/attachments/1/2/review.png",
                "proxy_url": "https://media.discordapp.net/attachments/1/2/review.png",
                "height": 128,
                "width": 256
            }
        ],
        "embeds": [
            {
                "type": "rich",
                "title": "Hello",
                "description": "World",
                "color": 3447003,
                "footer": { "text": "footer text" },
                "fields": [ { "name": "n", "value": "v", "inline": true } ]
            }
        ],
        "reactions": [
            { "count": 2, "me": true, "emoji": { "id": null, "name": "🔥" } },
            { "count": 1, "me": false, "emoji": { "id": "417719834621489152", "name": "ultra_daruma" } }
        ],
        "pinned": false,
        "type": 19,
        "flags": 4,
        "message_reference": {
            "type": 0,
            "message_id": "1021934727088820223",
            "channel_id": "838309323191025706",
            "guild_id": "838309323191025704",
            "fail_if_not_exists": true
        }
    }"#;

    #[test]
    fn message_deserializes() {
        let message: Message = serde_json::from_str(MESSAGE).unwrap();
        assert_eq!(message.id.get(), 1_021_934_727_088_820_224);
        assert_eq!(message.channel_id.get(), 838_309_323_191_025_706);
        assert_eq!(message.author.username, "Nelly");
        assert_eq!(message.timestamp.as_datetime().year(), 2022);
        assert!(message.edited_timestamp.is_some());
        assert!(!message.tts);
        assert_eq!(message.mentions.len(), 1);
        assert_eq!(message.attachments.len(), 1);
        assert_eq!(message.attachments[0].filename, "review.png");
        assert_eq!(message.attachments[0].size, 12_345);
        assert_eq!(message.attachments[0].height, Some(128));
        assert_eq!(message.embeds.len(), 1);
        assert_eq!(message.embeds[0].title.as_deref(), Some("Hello"));
        assert_eq!(message.embeds[0].color, Some(3_447_003));
        let fields = message.embeds[0].fields.as_ref().unwrap();
        assert!(fields[0].inline.unwrap_or(false));
        let reactions = message.reactions.as_ref().unwrap();
        assert_eq!(reactions.len(), 2);
        assert!(reactions[0].me);
        assert_eq!(reactions[1].emoji.name.as_deref(), Some("ultra_daruma"));
        assert_eq!(message.r#type, MessageType::Reply);
        assert_eq!(message.flags, Some(MessageFlags::SUPPRESS_EMBEDS));
        let reference = message.message_reference.unwrap();
        assert_eq!(
            reference.message_id.map(|id| id.get()),
            Some(1_021_934_727_088_820_223)
        );
    }

    #[test]
    fn minimal_message() {
        // Message objects vary in which fields Discord populates.
        let message: Message = serde_json::from_str(
            r#"{
                "id": "1021934727088820224",
                "channel_id": "838309323191025706",
                "author": { "id": "80351110224678912", "username": "N", "discriminator": "0" },
                "content": "",
                "timestamp": "2022-09-22T14:00:00.436000+00:00",
                "tts": false,
                "mention_everyone": false,
                "mentions": [],
                "mention_roles": [],
                "attachments": [],
                "embeds": [],
                "pinned": false,
                "type": 0
            }"#,
        )
        .unwrap();
        assert_eq!(message.r#type, MessageType::Default);
        assert_eq!(message.reactions, None);
        assert_eq!(message.flags, None);
        assert_eq!(message.webhook_id, None);
    }

    #[test]
    fn components_v2_text_and_media_are_read() {
        let message: Message = serde_json::from_value(serde_json::json!({
            "id": "1", "channel_id": "2", "author": { "id": "3", "username": "statbot", "discriminator": "0", "bot": true },
            "content": "", "timestamp": "2026-10-31T22:30:00+00:00", "tts": false, "mention_everyone": false,
            "mentions": [], "mention_roles": [], "attachments": [], "embeds": [], "pinned": false, "type": 0,
            "flags": 32768,
            "components": [{ "type": 17, "components": [
                { "type": 10, "content": "# Messages" },
                { "type": 12, "items": [ { "media": { "url": "https://cdn.example/stats.png" } } ] },
                { "type": 9, "components": [ { "type": 10, "content": "198 messages" } ],
                  "accessory": { "type": 11, "media": { "url": "https://cdn.example/thumb.png" } } }
            ] }]
        }))
        .unwrap();
        assert_eq!(message.full_text(), "# Messages\n198 messages");
        assert_eq!(
            message.media_urls(),
            vec![
                "https://cdn.example/stats.png",
                "https://cdn.example/thumb.png"
            ]
        );
    }

    #[test]
    fn unknown_message_type_reads_as_default() {
        // 13 is a gap in the documented message type table. The message
        // must still parse: dropping a whole MESSAGE_CREATE because Discord
        // added a system message type would silently lose messages.
        let message = serde_json::from_str::<Message>(
            r#"{ "id": "1", "channel_id": "2", "author": { "id": "3", "username": "x", "discriminator": "0" },
                 "content": "", "timestamp": "2022-09-22T14:00:00.436000+00:00", "tts": false,
                 "mention_everyone": false, "mentions": [], "mention_roles": [], "attachments": [],
                 "embeds": [], "pinned": false, "type": 13 }"#,
        )
        .unwrap();
        assert_eq!(message.r#type, MessageType::Default);
    }

    #[test]
    fn message_flags_lenient() {
        let flags: MessageFlags = serde_json::from_str("1048576").unwrap(); // 1 << 20, undocumented
        assert_eq!(flags.bits(), 0);
        let flags: MessageFlags = serde_json::from_str("7").unwrap();
        assert_eq!(
            flags,
            MessageFlags::CROSSPOSTED | MessageFlags::IS_CROSSPOST | MessageFlags::SUPPRESS_EMBEDS
        );
    }

    #[test]
    fn embed_roundtrip_for_requests() {
        let embed = Embed {
            title: Some("Title".to_owned()),
            description: Some("Description".to_owned()),
            color: Some(0x00FF_00FF),
            fields: Some(vec![EmbedField {
                name: "a".to_owned(),
                value: "b".to_owned(),
                inline: Some(true),
            }]),
            ..Embed::default()
        };
        let json = serde_json::to_string(&embed).unwrap();
        // Unset fields must not be serialized as nulls in request bodies.
        assert!(!json.contains("null"));
        assert!(json.contains(r#""title":"Title""#));
        let reparsed: Embed = serde_json::from_str(&json).unwrap();
        assert_eq!(reparsed, embed);
    }

    #[test]
    fn message_roundtrips() {
        let message: Message = serde_json::from_str(MESSAGE).unwrap();
        let json = serde_json::to_string(&message).unwrap();
        let reparsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(reparsed, message);
    }

    #[test]
    fn message_reference_default() {
        let reference = MessageReference::default();
        assert_eq!(reference.message_id, None);
        let json = serde_json::to_string(&reference).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn partial_message_tolerates_sparse_payloads() {
        // MESSAGE_UPDATE may carry only a handful of fields.
        let message: PartialMessage = serde_json::from_str(
            r#"{
                "id": "1021934727088820224",
                "channel_id": "838309323191025706",
                "content": "edited!",
                "author": { "id": "80351110224678912", "username": "N", "discriminator": "0" }
            }"#,
        )
        .unwrap();
        assert_eq!(message.content.as_deref(), Some("edited!"));
        assert_eq!(message.guild_id, None);
        assert_eq!(message.embeds, None);
        assert_eq!(message.r#type, None);
    }

    #[test]
    fn message_delete_payloads() {
        let delete: MessageDelete =
            serde_json::from_str(r#"{ "id": "1", "channel_id": "2", "guild_id": "3" }"#).unwrap();
        assert_eq!(delete.id.get(), 1);
        assert_eq!(delete.guild_id.map(|g| g.get()), Some(3));

        let bulk: MessageDeleteBulk =
            serde_json::from_str(r#"{ "ids": ["1", "2"], "channel_id": "2" }"#).unwrap();
        assert_eq!(bulk.ids.len(), 2);
        assert_eq!(bulk.guild_id, None);
    }

    #[test]
    fn reaction_payloads() {
        let add: MessageReactionAdd = serde_json::from_str(
            r#"{
                "user_id": "42",
                "channel_id": "2",
                "message_id": "3",
                "guild_id": "4",
                "member": { "roles": [], "deaf": false, "mute": false },
                "emoji": { "id": null, "name": "🔥" },
                "burst": false,
                "type": 0
            }"#,
        )
        .unwrap();
        assert_eq!(add.user_id.get(), 42);
        assert_eq!(add.emoji.name.as_deref(), Some("🔥"));
        assert!(add.member.is_some());
        assert_eq!(add.message_author_id, None);

        let remove: MessageReactionRemove = serde_json::from_str(
            r#"{
                "user_id": "42",
                "channel_id": "2",
                "message_id": "3",
                "emoji": { "id": "417719834621489152", "name": "ultra_daruma" },
                "burst": true,
                "type": 1
            }"#,
        )
        .unwrap();
        assert!(remove.burst);
        assert!(remove.guild_id.is_none());
    }
}
