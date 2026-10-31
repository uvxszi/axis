//! Endpoint methods on [`Http`], returning the typed request builders
//! from [`crate::builders`].

use axis_core::{ChannelId, GenericId, GuildId, MessageId, RoleId, UserId};
use axis_model::{Emoji, OverwriteType};

use crate::{
    builders::{
        AddReaction, BanMember, BulkDeleteMessages, ClearReactions, CreateGuildChannel,
        CreateMessage, CreateRole, DeleteChannel, DeleteChannelPermission, DeleteMessage,
        DeleteOwnReaction, DeleteRole, DeleteUserReaction, EditChannel, EditChannelPermission,
        EditMember, EditMessage, EditRole, GetChannel, GetGuild, GetMember, GetMessage,
        GetMessages, KickMember, UnbanMember,
    },
    client::Http,
};

impl Http {
    /// Send a message to a channel.
    pub fn create_message(&self, channel_id: ChannelId) -> CreateMessage<'_> {
        CreateMessage::new(self, channel_id)
    }

    /// Edit a message previously sent by the bot (or edit others with the
    /// right permissions, per Discord's rules).
    pub fn edit_message(&self, channel_id: ChannelId, message_id: MessageId) -> EditMessage<'_> {
        EditMessage::new(self, channel_id, message_id)
    }

    /// Delete a message.
    pub fn delete_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> DeleteMessage<'_> {
        DeleteMessage::new(self, channel_id, message_id)
    }

    /// Fetch a single message.
    pub fn get_message(&self, channel_id: ChannelId, message_id: MessageId) -> GetMessage<'_> {
        GetMessage::new(self, channel_id, message_id)
    }

    /// List messages in a channel, with pagination parameters.
    pub fn get_messages(&self, channel_id: ChannelId) -> GetMessages<'_> {
        GetMessages::new(self, channel_id)
    }

    /// Bulk-delete 2-100 messages in a guild channel.
    pub fn bulk_delete_messages(
        &self,
        channel_id: ChannelId,
        message_ids: Vec<MessageId>,
    ) -> BulkDeleteMessages<'_> {
        BulkDeleteMessages::new(self, channel_id, message_ids)
    }

    /// Fetch a channel.
    pub fn get_channel(&self, channel_id: ChannelId) -> GetChannel<'_> {
        GetChannel::new(self, channel_id)
    }

    /// Create a channel in a guild.
    pub fn create_guild_channel(
        &self,
        guild_id: GuildId,
        name: impl Into<String>,
    ) -> CreateGuildChannel<'_> {
        CreateGuildChannel::new(self, guild_id, name)
    }

    /// Edit a channel.
    pub fn edit_channel(&self, channel_id: ChannelId) -> EditChannel<'_> {
        EditChannel::new(self, channel_id)
    }

    /// Delete a channel. Returns the deleted channel object.
    pub fn delete_channel(&self, channel_id: ChannelId) -> DeleteChannel<'_> {
        DeleteChannel::new(self, channel_id)
    }

    /// Create or replace a permission overwrite on a channel.
    pub fn edit_channel_permission(
        &self,
        channel_id: ChannelId,
        overwrite_id: GenericId,
        overwrite_type: OverwriteType,
    ) -> EditChannelPermission<'_> {
        EditChannelPermission::new(self, channel_id, overwrite_id, overwrite_type)
    }

    /// Delete a permission overwrite from a channel.
    pub fn delete_channel_permission(
        &self,
        channel_id: ChannelId,
        overwrite_id: GenericId,
    ) -> DeleteChannelPermission<'_> {
        DeleteChannelPermission::new(self, channel_id, overwrite_id)
    }

    /// Fetch a guild.
    pub fn get_guild(&self, guild_id: GuildId) -> GetGuild<'_> {
        GetGuild::new(self, guild_id)
    }

    /// Fetch a guild member.
    pub fn get_member(&self, guild_id: GuildId, user_id: UserId) -> GetMember<'_> {
        GetMember::new(self, guild_id, user_id)
    }

    /// Kick a member from a guild.
    pub fn kick_member(&self, guild_id: GuildId, user_id: UserId) -> KickMember<'_> {
        KickMember::new(self, guild_id, user_id)
    }

    /// Ban a member from a guild.
    pub fn ban_member(&self, guild_id: GuildId, user_id: UserId) -> BanMember<'_> {
        BanMember::new(self, guild_id, user_id)
    }

    /// Lift a ban.
    pub fn unban_member(&self, guild_id: GuildId, user_id: UserId) -> UnbanMember<'_> {
        UnbanMember::new(self, guild_id, user_id)
    }

    /// Edit a guild member (nickname, roles, mute/deafen, timeout).
    pub fn edit_member(&self, guild_id: GuildId, user_id: UserId) -> EditMember<'_> {
        EditMember::new(self, guild_id, user_id)
    }

    /// Create a guild role.
    pub fn create_role(&self, guild_id: GuildId) -> CreateRole<'_> {
        CreateRole::new(self, guild_id)
    }

    /// Edit a guild role.
    pub fn edit_role(&self, guild_id: GuildId, role_id: RoleId) -> EditRole<'_> {
        EditRole::new(self, guild_id, role_id)
    }

    /// Delete a guild role.
    pub fn delete_role(&self, guild_id: GuildId, role_id: RoleId) -> DeleteRole<'_> {
        DeleteRole::new(self, guild_id, role_id)
    }

    /// React to a message as the bot.
    pub fn add_reaction(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: Emoji,
    ) -> AddReaction<'_> {
        AddReaction::new(self, channel_id, message_id, emoji)
    }

    /// Remove the bot's own reaction from a message.
    pub fn delete_own_reaction(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: Emoji,
    ) -> DeleteOwnReaction<'_> {
        DeleteOwnReaction::new(self, channel_id, message_id, emoji)
    }

    /// Remove a specific user's reaction from a message.
    pub fn delete_user_reaction(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: Emoji,
        user_id: UserId,
    ) -> DeleteUserReaction<'_> {
        DeleteUserReaction::new(self, channel_id, message_id, emoji, user_id)
    }

    /// Remove all reactions on a message.
    pub fn clear_reactions(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> ClearReactions<'_> {
        ClearReactions::new(self, channel_id, message_id)
    }
}
