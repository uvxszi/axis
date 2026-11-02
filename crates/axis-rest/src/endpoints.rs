//! Endpoint methods on [`Http`], returning the typed request builders
//! from [`crate::builders`].

use axis_core::{
    ApplicationCommandId, ApplicationId, ChannelId, GenericId, GuildId, InteractionId, MessageId,
    RoleId, UserId, error::Result,
};
use axis_model::{
    ApplicationCommandFields, ApplicationCommandOptionChoice, ApplicationCommandPermission, Emoji,
    InteractionMessageCallbackData, OverwriteType,
};

use crate::{
    builders::{
        AddReaction, AddThreadMember, BanMember, BulkDeleteMessages, BulkOverwriteGlobalCommands,
        BulkOverwriteGuildCommands, ClearReactions, CreateDm, CreateFollowup, CreateForumPost,
        CreateGlobalCommand, CreateGuildChannel, CreateInvite, CreateMessage, CreateRole,
        DeferInteraction, DeleteChannel, DeleteChannelPermission, DeleteFollowup,
        DeleteGlobalCommand, DeleteGuildCommand, DeleteMessage, DeleteOwnReaction, DeleteRole,
        DeleteUserReaction, EditChannel, EditChannelPermission, EditCurrentMember, EditFollowup,
        EditMember, EditMessage, EditRole, GetActiveThreads, GetAuditLog, GetChannel,
        GetCurrentUser, GetGlobalCommands, GetGuild, GetGuildChannels, GetGuildCommandPermissions,
        GetMember, GetMessage, GetMessages, GetPinnedMessages, GetReactions, GetUser, JoinThread,
        KickMember, ListMembers, PinMessage, RemoveThreadMember, RespondToAutocomplete,
        RespondToInteraction, SearchMembers, SetGuildCommandPermissions, StartThread,
        StartThreadFromMessage, TriggerTyping, UnbanMember, UnpinMessage,
    },
    client::Http,
    routing::Route,
};

impl Http {
    /// Fetch the recommended gateway URL (`GET /gateway`), e.g.
    /// `wss://gateway.discord.gg`.
    pub async fn get_gateway(&self) -> Result<String> {
        #[derive(serde::Deserialize)]
        struct GatewayResponse {
            url: String,
        }

        let route = Route::new(reqwest::Method::GET, "/gateway", "/gateway".to_owned());
        let response: GatewayResponse = self.json(route, None, None).await?;
        Ok(response.url)
    }

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

    /// List all channels in a guild.
    pub fn get_guild_channels(&self, guild_id: GuildId) -> GetGuildChannels<'_> {
        GetGuildChannels::new(self, guild_id)
    }

    /// Show a typing indicator in a channel.
    pub fn trigger_typing(&self, channel_id: ChannelId) -> TriggerTyping<'_> {
        TriggerTyping::new(self, channel_id)
    }

    /// Remove all reactions on a message.
    pub fn clear_reactions(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
    ) -> ClearReactions<'_> {
        ClearReactions::new(self, channel_id, message_id)
    }

    /// List the application's global commands.
    pub fn get_global_commands(&self, application_id: ApplicationId) -> GetGlobalCommands<'_> {
        GetGlobalCommands::new(self, application_id)
    }

    /// Create a global application command.
    pub fn create_global_command(
        &self,
        application_id: ApplicationId,
        fields: ApplicationCommandFields,
    ) -> CreateGlobalCommand<'_> {
        CreateGlobalCommand::new(self, application_id, fields)
    }

    /// Bulk-overwrite all global application commands — the recommended
    /// way to sync a bot's command set.
    pub fn bulk_overwrite_global_commands(
        &self,
        application_id: ApplicationId,
        commands: Vec<ApplicationCommandFields>,
    ) -> BulkOverwriteGlobalCommands<'_> {
        BulkOverwriteGlobalCommands::new(self, application_id, commands)
    }

    /// Delete a global application command.
    pub fn delete_global_command(
        &self,
        application_id: ApplicationId,
        command_id: ApplicationCommandId,
    ) -> DeleteGlobalCommand<'_> {
        DeleteGlobalCommand::new(self, application_id, command_id)
    }

    /// Bulk-overwrite all application commands in a guild.
    pub fn bulk_overwrite_guild_commands(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
        commands: Vec<ApplicationCommandFields>,
    ) -> BulkOverwriteGuildCommands<'_> {
        BulkOverwriteGuildCommands::new(self, application_id, guild_id, commands)
    }

    /// Delete a guild application command.
    pub fn delete_guild_command(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
        command_id: ApplicationCommandId,
    ) -> DeleteGuildCommand<'_> {
        DeleteGuildCommand::new(self, application_id, guild_id, command_id)
    }

    /// List a guild's application command permissions.
    pub fn get_guild_command_permissions(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
    ) -> GetGuildCommandPermissions<'_> {
        GetGuildCommandPermissions::new(self, application_id, guild_id)
    }

    /// Replace a command's per-guild permissions.
    pub fn set_guild_command_permissions(
        &self,
        application_id: ApplicationId,
        guild_id: GuildId,
        command_id: ApplicationCommandId,
        permissions: Vec<ApplicationCommandPermission>,
    ) -> SetGuildCommandPermissions<'_> {
        SetGuildCommandPermissions::new(self, application_id, guild_id, command_id, permissions)
    }

    /// Start a public thread from an existing message.
    pub fn start_thread_from_message(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        name: impl Into<String>,
    ) -> StartThreadFromMessage<'_> {
        StartThreadFromMessage::new(self, channel_id, message_id, name.into())
    }

    /// Start a thread without a starter message (public by default).
    pub fn start_thread(&self, channel_id: ChannelId, name: impl Into<String>) -> StartThread<'_> {
        StartThread::new(self, channel_id, name.into())
    }

    /// Create a post (a thread with a starter message) in a forum or media
    /// channel.
    pub fn create_forum_post(
        &self,
        channel_id: ChannelId,
        name: impl Into<String>,
    ) -> CreateForumPost<'_> {
        CreateForumPost::new(self, channel_id, name.into())
    }

    /// Join a thread as the bot.
    pub fn join_thread(&self, thread_id: ChannelId) -> JoinThread<'_> {
        JoinThread::new(self, thread_id)
    }

    /// Add a member to a thread.
    pub fn add_thread_member(&self, thread_id: ChannelId, user_id: UserId) -> AddThreadMember<'_> {
        AddThreadMember::new(self, thread_id, user_id)
    }

    /// Remove a member from a thread.
    pub fn remove_thread_member(
        &self,
        thread_id: ChannelId,
        user_id: UserId,
    ) -> RemoveThreadMember<'_> {
        RemoveThreadMember::new(self, thread_id, user_id)
    }

    /// List a guild's active threads.
    pub fn get_active_threads(&self, guild_id: GuildId) -> GetActiveThreads<'_> {
        GetActiveThreads::new(self, guild_id)
    }

    /// Pin a message.
    pub fn pin_message(&self, channel_id: ChannelId, message_id: MessageId) -> PinMessage<'_> {
        PinMessage::new(self, channel_id, message_id)
    }

    /// Unpin a message.
    pub fn unpin_message(&self, channel_id: ChannelId, message_id: MessageId) -> UnpinMessage<'_> {
        UnpinMessage::new(self, channel_id, message_id)
    }

    /// List a channel's pinned messages.
    pub fn get_pinned_messages(&self, channel_id: ChannelId) -> GetPinnedMessages<'_> {
        GetPinnedMessages::new(self, channel_id)
    }

    /// List guild members (paginated; needs the `GUILD_MEMBERS` intent).
    pub fn list_members(&self, guild_id: GuildId) -> ListMembers<'_> {
        ListMembers::new(self, guild_id)
    }

    /// Search guild members by username or nickname prefix.
    pub fn search_members(&self, guild_id: GuildId, query: impl Into<String>) -> SearchMembers<'_> {
        SearchMembers::new(self, guild_id, query.into())
    }

    /// Edit the bot's own member in a guild (e.g. its nickname).
    pub fn edit_current_member(&self, guild_id: GuildId) -> EditCurrentMember<'_> {
        EditCurrentMember::new(self, guild_id)
    }

    /// Fetch a user.
    pub fn get_user(&self, user_id: UserId) -> GetUser<'_> {
        GetUser::new(self, user_id)
    }

    /// Fetch the bot's own user.
    pub fn get_current_user(&self) -> GetCurrentUser<'_> {
        GetCurrentUser::new(self)
    }

    /// Open (or fetch) a DM channel with a user.
    pub fn create_dm(&self, recipient_id: UserId) -> CreateDm<'_> {
        CreateDm::new(self, recipient_id)
    }

    /// Read a guild's audit log.
    pub fn get_audit_log(&self, guild_id: GuildId) -> GetAuditLog<'_> {
        GetAuditLog::new(self, guild_id)
    }

    /// List users who reacted to a message with an emoji.
    pub fn get_reactions(
        &self,
        channel_id: ChannelId,
        message_id: MessageId,
        emoji: Emoji,
    ) -> GetReactions<'_> {
        GetReactions::new(self, channel_id, message_id, emoji)
    }

    /// Create an invite to a channel.
    pub fn create_invite(&self, channel_id: ChannelId) -> CreateInvite<'_> {
        CreateInvite::new(self, channel_id)
    }

    /// Respond to an interaction with a message (initial response).
    pub fn respond_to_interaction(
        &self,
        interaction_id: InteractionId,
        token: impl Into<String>,
        data: InteractionMessageCallbackData,
    ) -> RespondToInteraction<'_> {
        RespondToInteraction::new(self, interaction_id, token.into(), data)
    }

    /// Defer an interaction response; follow up later.
    pub fn defer_interaction(
        &self,
        interaction_id: InteractionId,
        token: impl Into<String>,
    ) -> DeferInteraction<'_> {
        DeferInteraction::new(self, interaction_id, token.into())
    }

    /// Answer an autocomplete request with suggested choices.
    pub fn respond_to_autocomplete(
        &self,
        interaction_id: InteractionId,
        token: impl Into<String>,
        choices: Vec<ApplicationCommandOptionChoice>,
    ) -> RespondToAutocomplete<'_> {
        RespondToAutocomplete::new(self, interaction_id, token.into(), choices)
    }

    /// Send a followup message for a (usually deferred) interaction.
    pub fn create_followup(
        &self,
        application_id: ApplicationId,
        token: impl Into<String>,
        data: InteractionMessageCallbackData,
    ) -> CreateFollowup<'_> {
        CreateFollowup::new(self, application_id, token.into(), data)
    }

    /// Edit a followup message; pass [`crate::builders::ORIGINAL_RESPONSE`]
    /// as the message ID to edit the interaction's initial response.
    pub fn edit_followup(
        &self,
        application_id: ApplicationId,
        token: impl Into<String>,
        message_id: impl core::fmt::Display,
        fields: crate::builders::FollowupEditFields,
    ) -> EditFollowup<'_> {
        EditFollowup::new(self, application_id, token.into(), message_id, fields)
    }

    /// Delete a followup message; pass [`crate::builders::ORIGINAL_RESPONSE`]
    /// as the message ID to delete the interaction's initial response.
    pub fn delete_followup(
        &self,
        application_id: ApplicationId,
        token: impl Into<String>,
        message_id: impl core::fmt::Display,
    ) -> DeleteFollowup<'_> {
        DeleteFollowup::new(self, application_id, token.into(), message_id)
    }
}
