//! Command invocations: typed argument access and response helpers.

use std::sync::Arc;

use futures_util::future::BoxFuture;
use serde_json::Value;

use axis_core::{
    ChannelId, GenericId, RoleId, UserId,
    error::{AxisError, Result},
};
use axis_model::{CommandData, CommandDataOption, Embed, Interaction, MessageFlags};
use axis_rest::Http;

/// One command invocation, handed to the handler registered for it.
///
/// Provides typed access to the supplied options ([`Invocation::arg`] /
/// [`Invocation::opt`]) and helpers for responding ([`Invocation::reply`],
/// [`Invocation::defer`], [`Invocation::followup`]).
#[derive(Clone)]
pub struct Invocation {
    /// The REST client, for responses and followups.
    pub http: Arc<Http>,
    /// The raw interaction.
    pub interaction: Interaction,
    /// The command data (present for command and autocomplete
    /// interactions).
    pub data: CommandData,
    /// The subcommand path taken to reach the handler, e.g. `["set"]` or
    /// `["channel", "add"]`; empty for top-level commands.
    pub path: Vec<String>,
}

impl std::fmt::Debug for Invocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Invocation")
            .field("command", &self.data.name)
            .field("path", &self.path)
            .field("interaction_id", &self.interaction.id)
            .finish_non_exhaustive()
    }
}

impl Invocation {
    /// The options visible at the handler's scope: the subcommand's
    /// options for nested handlers, the command's options at the top
    /// level.
    pub fn scope_options(&self) -> &[CommandDataOption] {
        let mut scope = self.data.options.as_slice();
        for segment in &self.path {
            let Some(next) = scope
                .iter()
                .find(|option| &option.name == segment)
                .and_then(|option| option.options.as_deref())
            else {
                return &[];
            };
            scope = next;
        }
        scope
    }

    /// A required option's typed value; a missing or wrongly-typed value
    /// is an error.
    pub fn arg<T: FromCommandValue>(&self, name: &str) -> Result<T> {
        self.opt::<T>(name)?
            .ok_or_else(|| AxisError::invalid_data(format!("missing required option {name:?}")))
    }

    /// An optional option's typed value.
    pub fn opt<T: FromCommandValue>(&self, name: &str) -> Result<Option<T>> {
        let Some(option) = self
            .scope_options()
            .iter()
            .find(|option| option.name == name)
        else {
            return Ok(None);
        };
        let Some(value) = option.value.as_ref() else {
            return Ok(None);
        };
        T::from_command_value(value).map(Some)
    }

    /// Start building the interaction's initial response (due within 3
    /// seconds). Await the builder to send it.
    pub fn reply(&self) -> ReplyBuilder<'_> {
        ReplyBuilder::new(self, false)
    }

    /// Start building a followup message, for deferred interactions.
    /// Await the builder to send it.
    pub fn followup(&self) -> ReplyBuilder<'_> {
        ReplyBuilder::new(self, true)
    }

    /// Acknowledge the interaction without content yet; the user sees a
    /// loading state while the followup is prepared.
    pub async fn defer(&self, ephemeral: bool) -> Result<()> {
        self.http
            .defer_interaction(self.interaction.id, self.interaction.token.clone())
            .ephemeral(ephemeral)
            .await
    }
}

/// Convert an interaction option value into a handler-side type.
///
/// Implemented for `String`, `i64`, `f64`, `bool`, and the typed snowflake
/// IDs (`UserId`, `ChannelId`, `RoleId`, and the generic ID for
/// mentionable options).
pub trait FromCommandValue: Sized {
    /// Parse the raw JSON value Discord sent for the option.
    fn from_command_value(value: &Value) -> Result<Self>;
}

fn invalid_value(value: &Value, expected: &str) -> AxisError {
    AxisError::invalid_data(format!("option value {value} is not {expected}"))
}

impl FromCommandValue for String {
    fn from_command_value(value: &Value) -> Result<Self> {
        value
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| invalid_value(value, "a string"))
    }
}

impl FromCommandValue for i64 {
    fn from_command_value(value: &Value) -> Result<Self> {
        value
            .as_i64()
            .ok_or_else(|| invalid_value(value, "an integer"))
    }
}

impl FromCommandValue for f64 {
    fn from_command_value(value: &Value) -> Result<Self> {
        value
            .as_f64()
            .ok_or_else(|| invalid_value(value, "a number"))
    }
}

impl FromCommandValue for bool {
    fn from_command_value(value: &Value) -> Result<Self> {
        value
            .as_bool()
            .ok_or_else(|| invalid_value(value, "a boolean"))
    }
}

macro_rules! id_from_command_value {
    ($($ty:ty),*) => {
        $(
            impl FromCommandValue for $ty {
                fn from_command_value(value: &Value) -> Result<Self> {
                    serde_json::from_value(value.clone())
                        .map_err(|_| invalid_value(value, "a snowflake ID"))
                }
            }
        )*
    };
}

id_from_command_value!(UserId, ChannelId, RoleId, GenericId);

/// Builder for an interaction response or followup message.
///
/// Awaiting it sends the response: `invocation.reply()` produces the
/// initial response (`CHANNEL_MESSAGE_WITH_SOURCE`), while
/// `invocation.followup()` posts a followup via the interaction's webhook
/// (after a defer).
#[derive(Debug)]
pub struct ReplyBuilder<'a> {
    invocation: &'a Invocation,
    data: axis_model::InteractionMessageCallbackData,
    followup: bool,
}

impl<'a> ReplyBuilder<'a> {
    pub(crate) fn new(invocation: &'a Invocation, followup: bool) -> Self {
        Self {
            invocation,
            data: axis_model::InteractionMessageCallbackData::default(),
            followup,
        }
    }

    /// Set the message content.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.data.content = Some(content.into());
        self
    }

    /// Attach embeds.
    pub fn embeds(mut self, embeds: impl IntoIterator<Item = Embed>) -> Self {
        self.data.embeds = Some(embeds.into_iter().collect());
        self
    }

    /// Send with text-to-speech.
    pub fn tts(mut self, tts: bool) -> Self {
        self.data.tts = Some(tts);
        self
    }

    /// Make the message ephemeral — visible only to the invoking user.
    pub fn ephemeral(mut self, ephemeral: bool) -> Self {
        let mut flags = self.data.flags.unwrap_or_else(MessageFlags::empty);
        flags.set(MessageFlags::EPHEMERAL, ephemeral);
        self.data.flags = Some(flags);
        self
    }

    async fn execute(self) -> Result<()> {
        if self.followup {
            self.invocation
                .http
                .create_followup(
                    self.invocation.interaction.application_id,
                    self.invocation.interaction.token.clone(),
                    self.data,
                )
                .await
                .map(|_| ())
        } else {
            self.invocation
                .http
                .respond_to_interaction(
                    self.invocation.interaction.id,
                    self.invocation.interaction.token.clone(),
                    self.data,
                )
                .await
        }
    }
}

impl<'a> core::future::IntoFuture for ReplyBuilder<'a> {
    type Output = Result<()>;
    type IntoFuture = BoxFuture<'a, Result<()>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}
