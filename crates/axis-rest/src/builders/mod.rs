//! Typed, awaitable request builders for the Discord REST API.
//!
//! Every endpoint method on [`Http`](crate::Http) returns a builder whose
//! setters configure the request; awaiting the builder executes it and
//! resolves to the typed response. (Builders implement
//! [`IntoFuture`](core::future::IntoFuture).)
//!
//! ```
//! # async fn demo(http: &axis_rest::Http) -> axis_core::Result<()> {
//! let message = http
//!     .create_message(axis_core::ChannelId::new(123))
//!     .content("Hello from Axis!")
//!     .await?;
//! # let _ = message;
//! # Ok(())
//! # }
//! ```

/// Implements `IntoFuture` for a builder by delegating to its inherent
/// `async fn execute(self)`.
macro_rules! into_future {
    ($name:ident<$lifetime:lifetime> => $output:ty) => {
        impl<$lifetime> core::future::IntoFuture for $name<$lifetime> {
            type Output = $output;
            type IntoFuture =
                core::pin::Pin<Box<dyn core::future::Future<Output = $output> + Send + $lifetime>>;

            fn into_future(self) -> Self::IntoFuture {
                std::boxed::Box::pin(self.execute())
            }
        }
    };
}

pub mod application_command;
pub mod channel;
pub mod community;
pub mod guild;
pub mod interaction;
pub mod member;
pub mod message;
pub mod reaction;
pub mod role;

pub use application_command::{
    BulkOverwriteGlobalCommands, BulkOverwriteGuildCommands, CreateGlobalCommand,
    DeleteGlobalCommand, DeleteGuildCommand, GetGlobalCommands, GetGuildCommandPermissions,
    SetGuildCommandPermissions,
};
pub use channel::{
    CreateGuildChannel, DeleteChannel, DeleteChannelPermission, EditChannel, EditChannelPermission,
    GetChannel, GetGuildChannels, TriggerTyping,
};
pub use community::{
    AddThreadMember, CreateDm, CreateForumPost, CreateInvite, EditCurrentMember, GetActiveThreads,
    GetAuditLog, GetCurrentUser, GetPinnedMessages, GetReactions, GetUser, JoinThread, ListMembers,
    PinMessage, RemoveThreadMember, SearchMembers, StartThread, StartThreadFromMessage,
    UnpinMessage,
};
pub use guild::GetGuild;
pub use interaction::{
    CreateFollowup, DeferInteraction, DeleteFollowup, EditFollowup, FollowupEditFields,
    ORIGINAL_RESPONSE, RespondToAutocomplete, RespondToInteraction,
};
pub use member::{BanMember, EditMember, GetMember, KickMember, UnbanMember};
pub use message::{
    BulkDeleteMessages, CreateMessage, DeleteMessage, EditMessage, GetMessage, GetMessages,
};
pub use reaction::{AddReaction, ClearReactions, DeleteOwnReaction, DeleteUserReaction};
pub use role::{CreateRole, DeleteRole, EditRole};
