//! The event dispatcher: typed handler registration and the run loop.

use std::{future::Future, sync::Arc};

use futures_util::{Stream, StreamExt, future::BoxFuture};

use axis_core::error::{AxisError, Result};
use axis_gateway::ShardEvent;

use crate::event::Event;

/// A typed handler for one event payload.
type TypedHandler<P> = Arc<dyn Fn(P) -> BoxFuture<'static, Result<()>> + Send + Sync>;
/// A handler receiving the raw event kinds Axis does not model yet.
type UnknownHandler =
    Arc<dyn Fn(String, serde_json::Value) -> BoxFuture<'static, Result<()>> + Send + Sync>;
/// A catch-all handler observing every event.
type AnyHandler = Arc<dyn Fn(Event) -> BoxFuture<'static, Result<()>> + Send + Sync>;

/// Default-capacity-safe handler tables, one list per event kind.
#[derive(Default)]
struct HandlerTables {
    any: Vec<AnyHandler>,
    unknown: Vec<UnknownHandler>,
    ready: Vec<TypedHandler<axis_model::Ready>>,
    message_create: Vec<TypedHandler<axis_model::Message>>,
    message_update: Vec<TypedHandler<axis_model::PartialMessage>>,
    message_delete: Vec<TypedHandler<axis_model::MessageDelete>>,
    message_delete_bulk: Vec<TypedHandler<axis_model::MessageDeleteBulk>>,
    reaction_add: Vec<TypedHandler<axis_model::MessageReactionAdd>>,
    reaction_remove: Vec<TypedHandler<axis_model::MessageReactionRemove>>,
    member_add: Vec<TypedHandler<axis_model::MemberAdd>>,
    member_update: Vec<TypedHandler<axis_model::MemberUpdate>>,
    member_remove: Vec<TypedHandler<axis_model::MemberRemove>>,
    guild_create: Vec<TypedHandler<axis_model::GuildCreate>>,
    guild_update: Vec<TypedHandler<axis_model::Guild>>,
    guild_delete: Vec<TypedHandler<axis_model::UnavailableGuild>>,
    channel_create: Vec<TypedHandler<axis_model::Channel>>,
    channel_update: Vec<TypedHandler<axis_model::Channel>>,
    channel_delete: Vec<TypedHandler<axis_model::Channel>>,
    role_create: Vec<TypedHandler<axis_model::GuildRoleChange>>,
    role_update: Vec<TypedHandler<axis_model::GuildRoleChange>>,
    role_delete: Vec<TypedHandler<axis_model::GuildRoleDelete>>,
    interaction_create: Vec<TypedHandler<axis_model::Interaction>>,
}

impl std::fmt::Debug for HandlerTables {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerTables")
            .field("any", &self.any.len())
            .field("unknown", &self.unknown.len())
            .field("ready", &self.ready.len())
            .field("message_create", &self.message_create.len())
            .field("message_update", &self.message_update.len())
            .field("message_delete", &self.message_delete.len())
            .field("message_delete_bulk", &self.message_delete_bulk.len())
            .field("reaction_add", &self.reaction_add.len())
            .field("reaction_remove", &self.reaction_remove.len())
            .field("member_add", &self.member_add.len())
            .field("member_update", &self.member_update.len())
            .field("member_remove", &self.member_remove.len())
            .field("guild_create", &self.guild_create.len())
            .field("guild_update", &self.guild_update.len())
            .field("guild_delete", &self.guild_delete.len())
            .field("channel_create", &self.channel_create.len())
            .field("channel_update", &self.channel_update.len())
            .field("channel_delete", &self.channel_delete.len())
            .field("role_create", &self.role_create.len())
            .field("role_update", &self.role_update.len())
            .field("role_delete", &self.role_delete.len())
            .field("interaction_create", &self.interaction_create.len())
            .finish()
    }
}

macro_rules! registration {
    ($($method:ident => $field:ident : $payload:ty),* $(,)?) => {
        impl Dispatcher {
            $(
                /// Register a handler for this event kind. Handlers run in
                /// registration order for each event.
                pub fn $method<F, Fut>(&mut self, handler: F) -> &mut Self
                where
                    F: Fn($payload) -> Fut + Send + Sync + 'static,
                    Fut: Future<Output = Result<()>> + Send + 'static,
                {
                    self.tables.$field.push(Arc::new(move |payload| {
                        Box::pin(handler(payload))
                    }));
                    self
                }
            )*
        }
    };
}

registration! {
    on_ready => ready: axis_model::Ready,
    on_message_create => message_create: axis_model::Message,
    on_message_update => message_update: axis_model::PartialMessage,
    on_message_delete => message_delete: axis_model::MessageDelete,
    on_message_delete_bulk => message_delete_bulk: axis_model::MessageDeleteBulk,
    on_reaction_add => reaction_add: axis_model::MessageReactionAdd,
    on_reaction_remove => reaction_remove: axis_model::MessageReactionRemove,
    on_member_add => member_add: axis_model::MemberAdd,
    on_member_update => member_update: axis_model::MemberUpdate,
    on_member_remove => member_remove: axis_model::MemberRemove,
    on_guild_create => guild_create: axis_model::GuildCreate,
    on_guild_update => guild_update: axis_model::Guild,
    on_guild_delete => guild_delete: axis_model::UnavailableGuild,
    on_channel_create => channel_create: axis_model::Channel,
    on_channel_update => channel_update: axis_model::Channel,
    on_channel_delete => channel_delete: axis_model::Channel,
    on_role_create => role_create: axis_model::GuildRoleChange,
    on_role_update => role_update: axis_model::GuildRoleChange,
    on_role_delete => role_delete: axis_model::GuildRoleDelete,
    on_interaction_create => interaction_create: axis_model::Interaction,
}

/// Routes parsed [`Event`]s to registered handlers.
///
/// Handlers are async and run sequentially in registration order for each
/// event; a handler that returns an error stops later handlers for that
/// event and the error is returned from [`Dispatcher::dispatch`]. The
/// [`Dispatcher::run`] loop logs handler errors and keeps going, so one
/// failed handler does not end a long-running bot.
#[derive(Debug, Default)]
pub struct Dispatcher {
    tables: HandlerTables,
}

impl Dispatcher {
    /// Create an empty dispatcher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a catch-all handler observing every event (including
    /// [`Event::Unknown`]). Runs before the typed handlers; the event is
    /// cloned per catch-all handler.
    pub fn on_any<F, Fut>(&mut self, handler: F) -> &mut Self
    where
        F: Fn(Event) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.tables
            .any
            .push(Arc::new(move |event| Box::pin(handler(event))));
        self
    }

    /// Register a handler for event kinds Axis does not model yet.
    pub fn on_unknown<F, Fut>(&mut self, handler: F) -> &mut Self
    where
        F: Fn(String, serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.tables
            .unknown
            .push(Arc::new(move |kind, data| Box::pin(handler(kind, data))));
        self
    }

    /// Dispatch one event to the registered handlers.
    ///
    /// The catch-all (`on_any`) handlers run first, then the typed
    /// handlers for the event's kind in registration order. The first
    /// handler error stops later handlers and is returned.
    pub async fn dispatch(&self, event: Event) -> Result<()> {
        for handler in &self.tables.any {
            handler(event.clone()).await?;
        }
        match event {
            Event::Ready(payload) => self.run_all(&self.tables.ready, payload).await?,
            Event::Resumed => {}
            Event::MessageCreate(payload) => {
                self.run_all(&self.tables.message_create, payload).await?
            }
            Event::MessageUpdate(payload) => {
                self.run_all(&self.tables.message_update, payload).await?
            }
            Event::MessageDelete(payload) => {
                self.run_all(&self.tables.message_delete, payload).await?
            }
            Event::MessageDeleteBulk(payload) => {
                self.run_all(&self.tables.message_delete_bulk, payload)
                    .await?
            }
            Event::ReactionAdd(payload) => self.run_all(&self.tables.reaction_add, payload).await?,
            Event::ReactionRemove(payload) => {
                self.run_all(&self.tables.reaction_remove, payload).await?
            }
            Event::MemberAdd(payload) => self.run_all(&self.tables.member_add, payload).await?,
            Event::MemberUpdate(payload) => {
                self.run_all(&self.tables.member_update, payload).await?
            }
            Event::MemberRemove(payload) => {
                self.run_all(&self.tables.member_remove, payload).await?
            }
            Event::GuildCreate(payload) => self.run_all(&self.tables.guild_create, payload).await?,
            Event::GuildUpdate(payload) => self.run_all(&self.tables.guild_update, payload).await?,
            Event::GuildDelete(payload) => self.run_all(&self.tables.guild_delete, payload).await?,
            Event::ChannelCreate(payload) => {
                self.run_all(&self.tables.channel_create, payload).await?
            }
            Event::ChannelUpdate(payload) => {
                self.run_all(&self.tables.channel_update, payload).await?
            }
            Event::ChannelDelete(payload) => {
                self.run_all(&self.tables.channel_delete, payload).await?
            }
            Event::RoleCreate(payload) => self.run_all(&self.tables.role_create, payload).await?,
            Event::RoleUpdate(payload) => self.run_all(&self.tables.role_update, payload).await?,
            Event::RoleDelete(payload) => self.run_all(&self.tables.role_delete, payload).await?,
            Event::InteractionCreate(payload) => {
                self.run_all(&self.tables.interaction_create, payload)
                    .await?
            }
            Event::Unknown { kind, data } => {
                for handler in &self.tables.unknown {
                    handler(kind.clone(), data.clone()).await?;
                }
            }
        }
        Ok(())
    }

    /// Consume a shard's event stream, dispatching every event.
    ///
    /// Stream errors are fatal (the gateway gave up) and end the loop with
    /// the error; parse failures for known event kinds and handler errors
    /// are logged and skipped, keeping the bot alive.
    pub async fn run<S>(&self, events: S) -> Result<()>
    where
        S: Stream<Item = Result<ShardEvent, AxisError>> + Unpin,
    {
        let mut events = std::pin::pin!(events);
        while let Some(item) = events.next().await {
            match item {
                Ok(ShardEvent::Dispatch { kind, data }) => match Event::parse(&kind, data) {
                    Ok(event) => {
                        if let Err(error) = self.dispatch(event).await {
                            tracing::error!(%error, "event handler failed");
                        }
                    }
                    Err(error) => {
                        tracing::error!(kind = %kind, %error, "failed to parse gateway event; skipping");
                    }
                },
                Ok(_) => {
                    // Shard lifecycle signals (Connected / Resumed /
                    // Reconnecting) are surfaced by the gateway itself;
                    // dispatch-level equivalents arrive as READY/RESUMED.
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    async fn run_all<P: Clone>(&self, handlers: &[TypedHandler<P>], payload: P) -> Result<()> {
        for handler in handlers {
            handler(payload.clone()).await?;
        }
        Ok(())
    }
}
