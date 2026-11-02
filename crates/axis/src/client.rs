//! The client: the composition of the REST layer, the gateway shard, and
//! the event dispatcher — the entry point applications interact with.

use std::{sync::Arc, time::Duration};

use axis_commands::CommandRegistry;
use axis_core::{
    GatewayIntents, Token,
    error::{AxisError, Result},
};
use axis_events::{Dispatcher, Event};
use axis_rest::Http;

/// The context handed to event handlers: access to the REST client (and,
/// in later phases, cached state).
#[derive(Clone, Debug)]
pub struct Context {
    /// The REST client, for responding to events.
    pub http: Arc<Http>,
}

/// The Axis client: one gateway shard plus the REST client and event
/// dispatch.
///
/// ```
/// use axis::prelude::*;
/// # async fn demo() -> Result<(), AxisError> {
///
/// let mut client = Client::builder(Token::from_env("DISCORD_TOKEN")?)
///     .intents(GatewayIntents::unprivileged() | GatewayIntents::MESSAGE_CONTENT)
///     .build()
///     .await?;
///
/// client.on_message_create(|ctx, message| async move {
///     if message.content == "!hello" {
///         ctx.http
///             .create_message(message.channel_id)
///             .content("Hello from Axis!")
///             .await?;
///     }
///     Ok(())
/// });
///
/// client.start().await?;
/// # Ok(())
/// # }
/// ```
///
/// `build` fetches the recommended gateway URL via `GET /gateway` (skip
/// that with [`ClientBuilder::gateway_url`], e.g. in tests); `start` runs
/// until the shard stops. Pair it with `tokio::select!` and
/// [`Client::shutdown`] for a graceful stop.
#[derive(Debug)]
pub struct Client {
    http: Arc<Http>,
    shard: axis_gateway::Shard,
    dispatcher: Dispatcher,
    commands: Option<Arc<CommandRegistry>>,
}

/// Builder for a [`Client`].
#[derive(Debug)]
pub struct ClientBuilder {
    token: Token,
    intents: GatewayIntents,
    shard: Option<[u64; 2]>,
    gateway_url: Option<String>,
    api_base: Option<String>,
    timeout: Option<Duration>,
    max_reconnect_attempts: Option<u32>,
    commands: Option<CommandRegistry>,
    presence: Option<axis_gateway::Presence>,
    large_threshold: Option<u8>,
}

impl Client {
    /// Create a client builder.
    pub fn builder(token: Token) -> ClientBuilder {
        ClientBuilder::new(token)
    }

    /// The REST client, for calls outside event handlers.
    pub fn http(&self) -> Arc<Http> {
        Arc::clone(&self.http)
    }

    /// The command registry, when one was attached.
    pub fn commands(&self) -> Option<&CommandRegistry> {
        self.commands.as_deref()
    }

    /// Start the gateway shard and dispatch events until the shard stops.
    ///
    /// Runs until a fatal gateway error ends the shard (the error is
    /// returned) or the stream ends after [`Client::shutdown`].
    pub async fn start(&self) -> Result<()> {
        let events = self.shard.start();
        self.dispatcher.run(events).await
    }

    /// Run the dispatcher over an externally provided event stream
    /// instead of starting a shard — for custom transports and tests.
    pub async fn run_with<S>(&self, events: S) -> Result<()>
    where
        S: futures_util::Stream<Item = Result<axis_gateway::ShardEvent, AxisError>> + Unpin,
    {
        self.dispatcher.run(events).await
    }

    /// Signal a graceful shutdown: the shard closes with code 1000 and
    /// the event stream ends.
    pub fn shutdown(&self) {
        self.shard.shutdown();
    }

    /// A cloneable handle for gateway commands: presence updates, voice
    /// state updates, and guild member requests.
    pub fn messenger(&self) -> axis_gateway::ShardMessenger {
        self.shard.messenger()
    }
}

impl ClientBuilder {
    fn new(token: Token) -> Self {
        Self {
            token,
            intents: GatewayIntents::unprivileged(),
            shard: None,
            gateway_url: None,
            api_base: None,
            timeout: None,
            max_reconnect_attempts: None,
            commands: None,
            presence: None,
            large_threshold: None,
        }
    }

    /// The presence to identify with (restored after every reconnect).
    pub fn presence(mut self, presence: axis_gateway::Presence) -> Self {
        self.presence = Some(presence);
        self
    }

    /// Member count (50–250) above which `GUILD_CREATE` omits offline
    /// members.
    pub fn large_threshold(mut self, threshold: u8) -> Self {
        self.large_threshold = Some(threshold);
        self
    }

    /// Set the gateway intents to subscribe with.
    ///
    /// Defaults to all unprivileged intents. Privileged intents
    /// (`GUILD_PRESENCES`, `GUILD_MEMBERS`, `MESSAGE_CONTENT`) must also
    /// be enabled in the Developer Portal.
    pub fn intents(mut self, intents: GatewayIntents) -> Self {
        self.intents = intents;
        self
    }

    /// Set the shard index/count pair for sharded bots.
    pub fn shard(mut self, id: u64, total: u64) -> Self {
        self.shard = Some([id, total]);
        self
    }

    /// Override the gateway URL, skipping the `GET /gateway` fetch —
    /// for tests against a mock server or fixed deployments.
    pub fn gateway_url(mut self, url: impl Into<String>) -> Self {
        self.gateway_url = Some(url.into());
        self
    }

    /// Override the REST API base URL — for tests against a mock server.
    pub fn rest_api_base(mut self, api_base: impl Into<String>) -> Self {
        self.api_base = Some(api_base.into());
        self
    }

    /// Set the REST request timeout.
    pub fn rest_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Cap the shard's consecutive reconnect attempts before it gives up.
    ///
    /// Defaults to unlimited: long-running bots should ride out Discord
    /// outages.
    pub fn max_reconnect_attempts(mut self, max: u32) -> Self {
        self.max_reconnect_attempts = Some(max);
        self
    }

    /// Attach a command registry: interactions are dispatched to it, and
    /// the registry can be registered with Discord through
    /// [`Client::http`].
    pub fn commands(mut self, registry: CommandRegistry) -> Self {
        self.commands = Some(registry);
        self
    }

    /// Build the client. Fetches the recommended gateway URL unless
    /// overridden; never identifies with the gateway.
    pub async fn build(self) -> Result<Client> {
        let mut rest = Http::builder(self.token.clone());
        if let Some(api_base) = self.api_base {
            rest = rest.api_base(api_base);
        }
        if let Some(timeout) = self.timeout {
            rest = rest.timeout(timeout);
        }
        let http = Arc::new(rest.build()?);

        let gateway_url = match self.gateway_url {
            Some(url) => url,
            None => http.get_gateway().await?,
        };

        let mut shard_builder =
            axis_gateway::Shard::builder(self.token, self.intents).gateway_url(gateway_url);
        if let Some([id, total]) = self.shard {
            shard_builder = shard_builder.shard(id, total);
        }
        if let Some(max) = self.max_reconnect_attempts {
            shard_builder = shard_builder.max_reconnect_attempts(max);
        }
        if let Some(presence) = self.presence {
            shard_builder = shard_builder.presence(presence);
        }
        if let Some(threshold) = self.large_threshold {
            shard_builder = shard_builder.large_threshold(threshold);
        }
        let shard = shard_builder.build()?;

        let commands = self.commands.map(Arc::new);
        let mut dispatcher = Dispatcher::new();
        if let Some(registry) = &commands {
            let registry = Arc::clone(registry);
            let http_for_commands = Arc::clone(&http);
            dispatcher.on_interaction_create(move |interaction| {
                let registry = Arc::clone(&registry);
                let http = Arc::clone(&http_for_commands);
                async move { registry.handle(http, interaction).await }
            });
        }

        Ok(Client {
            http,
            shard,
            dispatcher,
            commands,
        })
    }
}

macro_rules! client_registration {
    ($($method:ident => $dispatcher_method:ident : ($($param:ident : $payload:ty),*)),* $(,)?) => {
        impl Client {
            $(
                /// Register a handler for this event kind. Handlers run in
                /// registration order for each event, and receive a
                /// [`Context`] with the REST client.
                pub fn $method<F, Fut>(&mut self, handler: F) -> &mut Self
                where
                    F: Fn(Context, $($payload),*) -> Fut + Send + Sync + 'static,
                    Fut: std::future::Future<Output = Result<()>> + Send + 'static,
                {
                    let http = Arc::clone(&self.http);
                    self.dispatcher.$dispatcher_method(move |$($param),*| {
                        let ctx = Context {
                            http: Arc::clone(&http),
                        };
                        handler(ctx, $($param),*)
                    });
                    self
                }
            )*
        }
    };
}

client_registration! {
    on_ready => on_ready: (payload: axis_model::Ready),
    on_message_create => on_message_create: (payload: axis_model::Message),
    on_message_update => on_message_update: (payload: axis_model::PartialMessage),
    on_message_delete => on_message_delete: (payload: axis_model::MessageDelete),
    on_message_delete_bulk => on_message_delete_bulk: (payload: axis_model::MessageDeleteBulk),
    on_reaction_add => on_reaction_add: (payload: axis_model::MessageReactionAdd),
    on_reaction_remove => on_reaction_remove: (payload: axis_model::MessageReactionRemove),
    on_member_add => on_member_add: (payload: axis_model::MemberAdd),
    on_member_update => on_member_update: (payload: axis_model::MemberUpdate),
    on_member_remove => on_member_remove: (payload: axis_model::MemberRemove),
    on_guild_create => on_guild_create: (payload: axis_model::GuildCreate),
    on_guild_update => on_guild_update: (payload: axis_model::Guild),
    on_guild_delete => on_guild_delete: (payload: axis_model::UnavailableGuild),
    on_channel_create => on_channel_create: (payload: axis_model::Channel),
    on_channel_update => on_channel_update: (payload: axis_model::Channel),
    on_channel_delete => on_channel_delete: (payload: axis_model::Channel),
    on_role_create => on_role_create: (payload: axis_model::GuildRoleChange),
    on_role_update => on_role_update: (payload: axis_model::GuildRoleChange),
    on_role_delete => on_role_delete: (payload: axis_model::GuildRoleDelete),
    on_interaction_create => on_interaction_create: (payload: axis_model::Interaction),
    on_thread_create => on_thread_create: (payload: axis_model::Channel),
    on_thread_update => on_thread_update: (payload: axis_model::Channel),
    on_thread_delete => on_thread_delete: (payload: axis_model::Channel),
    on_voice_state_update => on_voice_state_update: (payload: axis_model::VoiceState),
    on_voice_server_update => on_voice_server_update: (payload: axis_model::VoiceServerUpdate),
    on_typing_start => on_typing_start: (payload: axis_model::TypingStart),
    on_guild_members_chunk => on_guild_members_chunk: (payload: axis_model::GuildMembersChunk),
    on_any => on_any: (event: Event),
    on_unknown => on_unknown: (kind: String, data: serde_json::Value),
}
