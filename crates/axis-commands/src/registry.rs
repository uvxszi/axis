//! The command registry: declaring, registering, and invoking commands.

use std::{future::Future, sync::Arc};

use futures_util::future::BoxFuture;

use axis_core::{
    ApplicationId, GuildId, Permissions,
    error::{AxisError, Result},
};
use axis_model::{
    ApplicationCommandFields, ApplicationCommandOption, ApplicationCommandOptionChoice,
    ApplicationCommandOptionType, ChannelType, CommandData, CommandDataOption, CommandType,
    Interaction, InteractionType,
};
use axis_rest::Http;

use crate::invocation::Invocation;

/// A command handler: receives the invocation, responds as it wishes.
pub type Handler = Arc<dyn Fn(Invocation) -> BoxFuture<'static, Result<()>> + Send + Sync>;

/// An autocomplete provider for one option: receives the invocation (with
/// the focused option's current input) and returns suggested choices.
pub type AutocompleteHandler = Arc<
    dyn Fn(Invocation) -> BoxFuture<'static, Result<Vec<ApplicationCommandOptionChoice>>>
        + Send
        + Sync,
>;

/// One declared option of a command or subcommand.
#[derive(Clone)]
pub struct CommandOption {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) kind: ApplicationCommandOptionType,
    pub(crate) required: bool,
    pub(crate) choices: Vec<ApplicationCommandOptionChoice>,
    pub(crate) autocomplete: Option<AutocompleteHandler>,
    pub(crate) channel_types: Vec<ChannelType>,
    pub(crate) min_value: Option<f64>,
    pub(crate) max_value: Option<f64>,
    pub(crate) min_length: Option<u16>,
    pub(crate) max_length: Option<u16>,
}

impl std::fmt::Debug for CommandOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandOption")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("required", &self.required)
            .field("choices", &self.choices.len())
            .field("has_autocomplete", &self.autocomplete.is_some())
            .finish_non_exhaustive()
    }
}

impl CommandOption {
    /// Declare a string option.
    pub fn string(name: impl Into<String>, description: impl Into<String>) -> OptionBuilder {
        OptionBuilder::new(ApplicationCommandOptionType::String, name, description)
    }

    /// Declare an integer option.
    pub fn integer(name: impl Into<String>, description: impl Into<String>) -> OptionBuilder {
        OptionBuilder::new(ApplicationCommandOptionType::Integer, name, description)
    }

    /// Declare a boolean option.
    pub fn boolean(name: impl Into<String>, description: impl Into<String>) -> OptionBuilder {
        OptionBuilder::new(ApplicationCommandOptionType::Boolean, name, description)
    }

    /// Declare a user option.
    pub fn user(name: impl Into<String>, description: impl Into<String>) -> OptionBuilder {
        OptionBuilder::new(ApplicationCommandOptionType::User, name, description)
    }

    /// Declare a channel option.
    pub fn channel(name: impl Into<String>, description: impl Into<String>) -> OptionBuilder {
        OptionBuilder::new(ApplicationCommandOptionType::Channel, name, description)
    }

    /// Declare a role option.
    pub fn role(name: impl Into<String>, description: impl Into<String>) -> OptionBuilder {
        OptionBuilder::new(ApplicationCommandOptionType::Role, name, description)
    }

    /// Declare a mentionable (user or role) option.
    pub fn mentionable(name: impl Into<String>, description: impl Into<String>) -> OptionBuilder {
        OptionBuilder::new(ApplicationCommandOptionType::Mentionable, name, description)
    }

    /// Declare a floating-point number option.
    pub fn number(name: impl Into<String>, description: impl Into<String>) -> OptionBuilder {
        OptionBuilder::new(ApplicationCommandOptionType::Number, name, description)
    }

    /// Declare an attachment option.
    pub fn attachment(name: impl Into<String>, description: impl Into<String>) -> OptionBuilder {
        OptionBuilder::new(ApplicationCommandOptionType::Attachment, name, description)
    }

    fn to_model(&self) -> ApplicationCommandOption {
        ApplicationCommandOption {
            r#type: self.kind,
            name: self.name.clone(),
            description: self.description.clone(),
            required: Some(self.required),
            // A registered autocomplete flag replaces fixed choices.
            choices: if self.autocomplete.is_some() || self.choices.is_empty() {
                None
            } else {
                Some(self.choices.clone())
            },
            options: None,
            channel_types: if self.channel_types.is_empty() {
                None
            } else {
                Some(self.channel_types.clone())
            },
            autocomplete: self.autocomplete.as_ref().map(|_| true),
            min_value: self.min_value,
            max_value: self.max_value,
            min_length: self.min_length,
            max_length: self.max_length,
        }
    }
}

/// Builder returned by the typed [`CommandOption`] constructors; chain
/// constraints and finish with [`OptionBuilder::finish`].
#[must_use = "an option builder does nothing until finished and added to a command"]
pub struct OptionBuilder {
    option: CommandOption,
}

impl OptionBuilder {
    fn new(
        kind: ApplicationCommandOptionType,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            option: CommandOption {
                name: name.into(),
                description: description.into(),
                kind,
                required: false,
                choices: Vec::new(),
                autocomplete: None,
                channel_types: Vec::new(),
                min_value: None,
                max_value: None,
                min_length: None,
                max_length: None,
            },
        }
    }

    /// Make the option required.
    pub fn required(mut self) -> Self {
        self.option.required = true;
        self
    }

    /// Add a fixed choice (string/integer/number options only; not
    /// allowed alongside autocomplete).
    pub fn choice(mut self, name: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.option.choices.push(ApplicationCommandOptionChoice {
            name: name.into(),
            name_localizations: None,
            value: value.into(),
        });
        self
    }

    /// Attach an autocomplete provider; the option becomes an autocomplete
    /// option (fixed choices are then not registered).
    pub fn autocomplete<F, Fut>(mut self, handler: F) -> Self
    where
        F: Fn(Invocation) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Vec<ApplicationCommandOptionChoice>>> + Send + 'static,
    {
        self.option.autocomplete = Some(Arc::new(move |invocation| Box::pin(handler(invocation))));
        self
    }

    /// Restrict a channel option to these channel kinds.
    pub fn channel_types(mut self, kinds: impl IntoIterator<Item = ChannelType>) -> Self {
        self.option.channel_types.extend(kinds);
        self
    }

    /// The smallest accepted value (integer/number options).
    pub fn min_value(mut self, min: f64) -> Self {
        self.option.min_value = Some(min);
        self
    }

    /// The largest accepted value (integer/number options).
    pub fn max_value(mut self, max: f64) -> Self {
        self.option.max_value = Some(max);
        self
    }

    /// The shortest accepted input (string options).
    pub fn min_length(mut self, min: u16) -> Self {
        self.option.min_length = Some(min);
        self
    }

    /// The longest accepted input (string options; at most 6000).
    pub fn max_length(mut self, max: u16) -> Self {
        self.option.max_length = Some(max);
        self
    }

    /// Finish the option for use with [`CommandBuilder::option`] or
    /// [`SubCommandBuilder::option`].
    pub fn finish(self) -> CommandOption {
        self.option
    }
}

/// A handler-less node in the command tree, used for subcommand groups.
#[derive(Clone)]
pub(crate) struct SubCommand {
    pub name: String,
    pub description: String,
    pub options: Vec<CommandOption>,
    pub handler: Option<Handler>,
}

/// The body of a registered command: either a leaf with its own options
/// and handler, or a branch of subcommands and groups.
#[derive(Clone)]
pub(crate) enum CommandNode {
    Leaf {
        options: Vec<CommandOption>,
        handler: Option<Handler>,
    },
    Branch {
        children: Vec<ChildEntry>,
    },
}

#[derive(Clone)]
pub(crate) enum ChildEntry {
    SubCommand(SubCommand),
    Group {
        name: String,
        description: String,
        subcommands: Vec<SubCommand>,
    },
}

/// A declared command, ready for registration and invocation.
#[derive(Clone)]
pub(crate) struct Command {
    pub name: String,
    pub description: String,
    pub default_member_permissions: Option<Permissions>,
    pub dm_permission: Option<bool>,
    pub nsfw: Option<bool>,
    pub node: CommandNode,
}

impl Command {
    /// The registration fields for this command.
    pub fn to_fields(&self) -> ApplicationCommandFields {
        let options = match &self.node {
            CommandNode::Leaf { options, .. } => {
                if options.is_empty() {
                    None
                } else {
                    Some(options.iter().map(CommandOption::to_model).collect())
                }
            }
            CommandNode::Branch { children } => Some(
                children
                    .iter()
                    .map(|child| match child {
                        ChildEntry::SubCommand(sub) => ApplicationCommandOption {
                            r#type: ApplicationCommandOptionType::SubCommand,
                            name: sub.name.clone(),
                            description: sub.description.clone(),
                            required: None,
                            choices: None,
                            options: if sub.options.is_empty() {
                                None
                            } else {
                                Some(sub.options.iter().map(CommandOption::to_model).collect())
                            },
                            channel_types: None,
                            autocomplete: None,
                            min_value: None,
                            max_value: None,
                            min_length: None,
                            max_length: None,
                        },
                        ChildEntry::Group {
                            name,
                            description,
                            subcommands,
                        } => ApplicationCommandOption {
                            r#type: ApplicationCommandOptionType::SubCommandGroup,
                            name: name.clone(),
                            description: description.clone(),
                            required: None,
                            choices: None,
                            options: Some(
                                subcommands
                                    .iter()
                                    .map(|sub| ApplicationCommandOption {
                                        r#type: ApplicationCommandOptionType::SubCommand,
                                        name: sub.name.clone(),
                                        description: sub.description.clone(),
                                        required: None,
                                        choices: None,
                                        options: if sub.options.is_empty() {
                                            None
                                        } else {
                                            Some(
                                                sub.options
                                                    .iter()
                                                    .map(CommandOption::to_model)
                                                    .collect(),
                                            )
                                        },
                                        channel_types: None,
                                        autocomplete: None,
                                        min_value: None,
                                        max_value: None,
                                        min_length: None,
                                        max_length: None,
                                    })
                                    .collect(),
                            ),
                            channel_types: None,
                            autocomplete: None,
                            min_value: None,
                            max_value: None,
                            min_length: None,
                            max_length: None,
                        },
                    })
                    .collect(),
            ),
        };

        ApplicationCommandFields {
            name: Some(self.name.clone()),
            description: Some(self.description.clone()),
            r#type: Some(CommandType::ChatInput),
            options,
            default_member_permissions: self.default_member_permissions,
            dm_permission: self.dm_permission,
            nsfw: self.nsfw,
        }
    }
}

/// The collection of commands a bot serves: declare them here, register
/// them with Discord, and dispatch incoming interactions.
///
/// ```
/// use axis_commands::CommandRegistry;
///
/// # fn demo() -> Result<(), axis_core::AxisError> {
/// let mut registry = CommandRegistry::new();
/// registry.command("echo", "Repeat after you", |cmd| {
///     cmd.option(
///         axis_commands::CommandOption::string("text", "What to echo")
///             .required()
///             .finish(),
///     )
///     .handler(|invocation| async move {
///         let text: String = invocation.arg("text")?;
///         invocation.reply().content(text).await?;
///         Ok(())
///     });
/// });
/// registry.validate()?;
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct CommandRegistry {
    commands: Vec<Command>,
}

impl std::fmt::Debug for CommandRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandRegistry")
            .field(
                "commands",
                &self
                    .commands
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl CommandRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare a command. The closure receives a builder; options,
    /// subcommands, groups, and the handler are configured inside it.
    ///
    /// A command is either a **leaf** (its own options plus a handler) or
    /// a **branch** (only subcommands and groups). Mixing both is rejected
    /// by [`CommandRegistry::validate`].
    pub fn command<F>(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        build: F,
    ) -> &mut Self
    where
        F: FnOnce(&mut CommandBuilder),
    {
        let mut builder = CommandBuilder {
            command: Command {
                name: name.into(),
                description: description.into(),
                default_member_permissions: None,
                dm_permission: None,
                nsfw: None,
                node: CommandNode::Leaf {
                    options: Vec::new(),
                    handler: None,
                },
            },
        };
        build(&mut builder);
        self.commands.push(builder.command);
        self
    }

    /// The number of declared commands.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether the registry has no commands.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Check the declared commands against the documented constraints:
    /// name and description lengths, required-before-optional option
    /// ordering, handler presence, and leaf/branch exclusivity.
    pub fn validate(&self) -> Result<()> {
        for command in &self.commands {
            validate_name(&command.name, "command")?;
            validate_description(&command.description, "command")?;
            match &command.node {
                CommandNode::Leaf { options, handler } => {
                    if handler.is_none() {
                        return Err(AxisError::invalid_data(format!(
                            "command {:?} has no handler",
                            command.name
                        )));
                    }
                    validate_options(options, &command.name)?;
                }
                CommandNode::Branch { children } => {
                    if children.is_empty() {
                        return Err(AxisError::invalid_data(format!(
                            "command {:?} declares no subcommands",
                            command.name
                        )));
                    }
                    for child in children {
                        match child {
                            ChildEntry::SubCommand(sub) => {
                                validate_sub(sub, &command.name)?;
                            }
                            ChildEntry::Group {
                                name, subcommands, ..
                            } => {
                                validate_name(name, "subcommand group")?;
                                if subcommands.is_empty() {
                                    return Err(AxisError::invalid_data(format!(
                                        "subcommand group {name:?} in command {:?} is empty",
                                        command.name
                                    )));
                                }
                                for sub in subcommands {
                                    validate_sub(sub, &command.name)?;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// The registration bodies for the declared commands, for use with
    /// the REST layer's bulk-overwrite endpoints.
    pub fn to_fields(&self) -> Vec<ApplicationCommandFields> {
        self.commands.iter().map(Command::to_fields).collect()
    }

    /// Sync the command set with Discord: bulk-overwrite all **global**
    /// commands with this registry's contents.
    pub async fn register_global(
        &self,
        http: &Http,
        application_id: ApplicationId,
    ) -> Result<Vec<axis_model::ApplicationCommand>> {
        http.bulk_overwrite_global_commands(application_id, self.to_fields())
            .await
    }

    /// Sync the command set with Discord for one guild only.
    pub async fn register_guild(
        &self,
        http: &Http,
        application_id: ApplicationId,
        guild_id: GuildId,
    ) -> Result<Vec<axis_model::ApplicationCommand>> {
        http.bulk_overwrite_guild_commands(application_id, guild_id, self.to_fields())
            .await
    }

    /// Dispatch an incoming interaction to the matching command handler,
    /// or answer an autocomplete request.
    pub async fn handle(&self, http: Arc<Http>, interaction: Interaction) -> Result<()> {
        match interaction.r#type {
            InteractionType::ApplicationCommand => self.invoke(http, interaction).await,
            InteractionType::ApplicationCommandAutocomplete => {
                self.autocomplete(http, interaction).await
            }
            other => Err(AxisError::invalid_data(format!(
                "{other:?} interactions are not commands"
            ))),
        }
    }

    async fn invoke(&self, http: Arc<Http>, interaction: Interaction) -> Result<()> {
        let data = match interaction.command_data() {
            Some(data) => data.clone(),
            None => {
                return Err(AxisError::invalid_data(
                    "command interaction is missing its data",
                ));
            }
        };

        let (_command, path, handler) = self.resolve(&data)?;
        let invocation = Invocation {
            http,
            interaction,
            data,
            path,
        };
        let Some(handler) = handler else {
            return Err(AxisError::invalid_data(format!(
                "command {:?} has no handler for this path",
                invocation.data.name
            )));
        };
        handler(invocation).await
    }

    async fn autocomplete(&self, http: Arc<Http>, interaction: Interaction) -> Result<()> {
        let data = match interaction.command_data() {
            Some(data) => data.clone(),
            None => {
                return Err(AxisError::invalid_data(
                    "autocomplete interaction is missing its data",
                ));
            }
        };

        // Capture identity up front: the interaction moves into the
        // invocation passed to the autocomplete handler.
        let interaction_id = interaction.id;
        let token = interaction.token.clone();

        // Find the focused option at the invocation's scope. The name is
        // cloned up front so the borrow of `data` ends before `data` moves
        // into the invocation.
        let (command, path, _handler) = self.resolve(&data)?;
        let focused_name = find_focused(&data, &path).map(|option| option.name.clone());
        let Some(focused_name) = focused_name else {
            tracing::warn!(
                command = %data.name,
                "autocomplete interaction without a focused option; answering with no choices"
            );
            return http
                .respond_to_autocomplete(interaction.id, interaction.token.clone(), Vec::new())
                .await;
        };

        let option = match &command.node {
            CommandNode::Leaf { options, .. } => options.iter().find(|o| o.name == focused_name),
            CommandNode::Branch { children } => children.iter().find_map(|child| match child {
                ChildEntry::SubCommand(sub)
                    if path.last().map(|p| p == &sub.name).unwrap_or(false) =>
                {
                    sub.options.iter().find(|o| o.name == focused_name)
                }
                ChildEntry::Group { subcommands, .. } if path.len() == 2 => subcommands
                    .iter()
                    .find(|sub| sub.name == path[1])
                    .and_then(|sub| sub.options.iter().find(|o| o.name == focused_name)),
                _ => None,
            }),
        };
        let Some(option) = option else {
            return Err(AxisError::invalid_data(format!(
                "autocomplete request references unknown option {:?}",
                focused_name
            )));
        };

        let Some(autocomplete) = option.autocomplete.as_ref() else {
            return Err(AxisError::invalid_data(format!(
                "option {:?} does not support autocomplete",
                focused_name
            )));
        };

        let invocation = Invocation {
            http: Arc::clone(&http),
            interaction,
            data,
            path,
        };
        let choices = autocomplete(invocation).await?;
        http.respond_to_autocomplete(interaction_id, token, choices)
            .await
    }

    /// Resolve a command interaction to its command, subcommand path, and
    /// handler.
    ///
    /// The returned command borrows only `self`, so the caller is free to
    /// move `data` afterwards.
    // The two lifetimes are deliberate: the returned command borrows only
    // `self`, so the caller may move `data` (eliding them would unify the
    // lifetimes and prevent that move).
    #[allow(clippy::needless_lifetimes)]
    fn resolve<'s, 'd>(
        &'s self,
        data: &'d CommandData,
    ) -> Result<(&'s Command, Vec<String>, Option<Handler>)> {
        let command = self
            .commands
            .iter()
            .find(|command| command.name == data.name)
            .ok_or_else(|| AxisError::invalid_data(format!("unknown command {:?}", data.name)))?;

        let mut path = Vec::new();

        let handler = match &command.node {
            CommandNode::Leaf { handler: leaf, .. } => leaf.clone(),
            CommandNode::Branch { children } => {
                // Chat-input commands nest at most one level: subcommands,
                // or groups of subcommands.
                let Some(first) = data.options.first() else {
                    return Err(AxisError::invalid_data(format!(
                        "command {:?} requires a subcommand or group",
                        data.name
                    )));
                };
                let Some(child) = children
                    .iter()
                    .find(|child| child_name(child) == first.name)
                else {
                    return Err(AxisError::invalid_data(format!(
                        "unknown subcommand {:?} for command {:?}",
                        first.name, data.name
                    )));
                };
                match child {
                    ChildEntry::Group {
                        name, subcommands, ..
                    } => {
                        if first.r#type != ApplicationCommandOptionType::SubCommandGroup as u8 {
                            return Err(AxisError::invalid_data(format!(
                                "expected subcommand group {:?}",
                                name
                            )));
                        }
                        path.push(name.clone());
                        let inner = first.options.as_deref().unwrap_or(&[]);
                        let Some(inner) = inner.first() else {
                            return Err(AxisError::invalid_data(format!(
                                "subcommand group {:?} received no subcommand",
                                name
                            )));
                        };
                        let Some(sub) = subcommands.iter().find(|sub| sub.name == inner.name)
                        else {
                            return Err(AxisError::invalid_data(format!(
                                "unknown subcommand {:?} in group {:?}",
                                inner.name, name
                            )));
                        };
                        path.push(sub.name.clone());
                        sub.handler.clone()
                    }
                    ChildEntry::SubCommand(sub) => {
                        if first.r#type != ApplicationCommandOptionType::SubCommand as u8 {
                            return Err(AxisError::invalid_data(format!(
                                "expected subcommand {:?}",
                                sub.name
                            )));
                        }
                        path.push(sub.name.clone());
                        sub.handler.clone()
                    }
                }
            }
        };

        Ok((command, path, handler))
    }
}

fn child_name(child: &ChildEntry) -> &str {
    match child {
        ChildEntry::SubCommand(sub) => &sub.name,
        ChildEntry::Group { name, .. } => name,
    }
}

fn find_focused<'a>(data: &'a CommandData, path: &[String]) -> Option<&'a CommandDataOption> {
    let mut scope = data.options.as_slice();
    for segment in path {
        scope = scope
            .iter()
            .find(|option| &option.name == segment)
            .and_then(|option| option.options.as_deref())?;
    }
    scope.iter().find(|option| option.focused.unwrap_or(false))
}

fn validate_name(name: &str, what: &str) -> Result<()> {
    let chars = name.chars().count();
    if !(1..=32).contains(&chars) {
        return Err(AxisError::invalid_data(format!(
            "{what} name must be 1-32 characters, got {chars}"
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return Err(AxisError::invalid_data(format!(
            "{what} name {name:?} may only contain lowercase letters, digits, hyphens, and underscores"
        )));
    }
    Ok(())
}

fn validate_description(description: &str, what: &str) -> Result<()> {
    let chars = description.chars().count();
    if !(1..=100).contains(&chars) {
        return Err(AxisError::invalid_data(format!(
            "{what} description must be 1-100 characters, got {chars}"
        )));
    }
    Ok(())
}

fn validate_options(options: &[CommandOption], command_name: &str) -> Result<()> {
    // The docs: required options must be listed before optional ones — a
    // required option after an optional one is the violation.
    let mut optional_seen = false;
    for option in options {
        validate_name(&option.name, "option")?;
        validate_description(&option.description, "option")?;
        if option.required && optional_seen {
            return Err(AxisError::invalid_data(format!(
                "command {command_name:?}: required options must be declared before optional ones (option {:?})",
                option.name
            )));
        }
        if !option.required {
            optional_seen = true;
        }
        if option.autocomplete.is_some() && !option.choices.is_empty() {
            return Err(AxisError::invalid_data(format!(
                "command {command_name:?}: option {:?} cannot combine choices with autocomplete",
                option.name
            )));
        }
    }
    Ok(())
}

fn validate_sub(sub: &SubCommand, command_name: &str) -> Result<()> {
    validate_name(&sub.name, "subcommand")?;
    validate_description(&sub.description, "subcommand")?;
    if sub.handler.is_none() {
        return Err(AxisError::invalid_data(format!(
            "subcommand {:?} in command {command_name:?} has no handler",
            sub.name
        )));
    }
    validate_options(&sub.options, command_name)
}

/// Builder passed to [`CommandRegistry::command`]'s closure.
#[must_use = "a command builder does nothing until its closure returns"]
pub struct CommandBuilder {
    command: Command,
}

impl CommandBuilder {
    /// Set the command's handler, making it a leaf command.
    pub fn handler<F, Fut>(&mut self, handler: F) -> &mut Self
    where
        F: Fn(Invocation) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler = Arc::new(move |invocation| Box::pin(handler(invocation)));
        match &mut self.command.node {
            CommandNode::Leaf { handler: slot, .. } => *slot = Some(handler),
            CommandNode::Branch { .. } => {
                // A branch command with a root handler is invalid; the
                // validate step reports it. Convert to keep the handler
                // visible rather than silently dropping it.
                self.command.node = CommandNode::Leaf {
                    options: Vec::new(),
                    handler: Some(handler),
                };
            }
        }
        self
    }

    /// Add an option to the command.
    pub fn option(&mut self, option: CommandOption) -> &mut Self {
        if let CommandNode::Leaf { options, .. } = &mut self.command.node {
            options.push(option);
        }
        self
    }

    /// Add a subcommand with its own options and handler.
    pub fn subcommand<F>(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        build: F,
    ) -> &mut Self
    where
        F: FnOnce(&mut SubCommandBuilder),
    {
        let mut builder = SubCommandBuilder {
            sub: SubCommand {
                name: name.into(),
                description: description.into(),
                options: Vec::new(),
                handler: None,
            },
        };
        build(&mut builder);
        let sub = builder.sub;
        match &mut self.command.node {
            CommandNode::Leaf { .. } => {
                self.command.node = CommandNode::Branch {
                    children: vec![ChildEntry::SubCommand(sub)],
                };
            }
            CommandNode::Branch { children } => children.push(ChildEntry::SubCommand(sub)),
        }
        self
    }

    /// Add a subcommand group containing further subcommands.
    pub fn group<F>(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        build: F,
    ) -> &mut Self
    where
        F: FnOnce(&mut GroupBuilder),
    {
        let mut builder = GroupBuilder {
            name: name.into(),
            description: description.into(),
            subcommands: Vec::new(),
        };
        build(&mut builder);
        let group = ChildEntry::Group {
            name: builder.name,
            description: builder.description,
            subcommands: builder.subcommands,
        };
        match &mut self.command.node {
            CommandNode::Leaf { .. } => {
                self.command.node = CommandNode::Branch {
                    children: vec![group],
                };
            }
            CommandNode::Branch { children } => children.push(group),
        }
        self
    }

    /// The default member permissions required to use the command.
    pub fn default_member_permissions(&mut self, permissions: Permissions) -> &mut Self {
        self.command.default_member_permissions = Some(permissions);
        self
    }

    /// Whether the command may be used in DMs (global commands only).
    pub fn dm_permission(&mut self, allowed: bool) -> &mut Self {
        self.command.dm_permission = Some(allowed);
        self
    }

    /// Mark the command as age-restricted.
    pub fn nsfw(&mut self, nsfw: bool) -> &mut Self {
        self.command.nsfw = Some(nsfw);
        self
    }
}

/// Builder passed to [`CommandBuilder::subcommand`]'s closure.
#[must_use = "a subcommand builder does nothing until its closure returns"]
pub struct SubCommandBuilder {
    sub: SubCommand,
}

impl SubCommandBuilder {
    /// Set the subcommand's handler.
    pub fn handler<F, Fut>(&mut self, handler: F) -> &mut Self
    where
        F: Fn(Invocation) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.sub.handler = Some(Arc::new(move |invocation| Box::pin(handler(invocation))));
        self
    }

    /// Add an option to the subcommand.
    pub fn option(&mut self, option: CommandOption) -> &mut Self {
        self.sub.options.push(option);
        self
    }
}

/// Builder passed to [`CommandBuilder::group`]'s closure.
#[must_use = "a group builder does nothing until its closure returns"]
pub struct GroupBuilder {
    name: String,
    description: String,
    subcommands: Vec<SubCommand>,
}

impl GroupBuilder {
    /// Add a subcommand to the group.
    pub fn subcommand<F>(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        build: F,
    ) -> &mut Self
    where
        F: FnOnce(&mut SubCommandBuilder),
    {
        let mut builder = SubCommandBuilder {
            sub: SubCommand {
                name: name.into(),
                description: description.into(),
                options: Vec::new(),
                handler: None,
            },
        };
        build(&mut builder);
        self.subcommands.push(builder.sub);
        self
    }
}
