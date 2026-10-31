//! # axis-commands
//!
//! An independent command framework for the Axis Discord library: declare
//! slash commands with typed options, subcommands, groups, permissions,
//! and autocomplete; register them with Discord; dispatch incoming
//! interactions.
//!
//! ```
//! use axis_commands::{CommandOption, CommandRegistry};
//!
//! # fn demo() -> Result<(), axis_core::AxisError> {
//! let mut registry = CommandRegistry::new();
//! registry.command("echo", "Repeat after you", |cmd| {
//!     cmd.option(
//!         CommandOption::string("text", "What to echo").required().finish(),
//!     )
//!     .handler(|invocation| async move {
//!         let text: String = invocation.arg("text")?;
//!         invocation.reply().content(text).ephemeral(true).await?;
//!         Ok(())
//!     });
//! })
//! .command("config", "Configure the bot", |cmd| {
//!     cmd.subcommand("set", "Set a value", |sub| {
//!         sub.option(
//!             CommandOption::string("key", "Which value").required().finish(),
//!         )
//!         .handler(|invocation| async move {
//!             invocation.defer(false).await?;
//!             invocation.followup().content("done").await?;
//!             Ok(())
//!         });
//!     });
//! });
//! registry.validate()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Flow
//!
//! 1. Declare commands on a [`CommandRegistry`] (validated by
//!    [`CommandRegistry::validate`]).
//! 2. Register with Discord once at startup:
//!    [`CommandRegistry::register_global`] or
//!    [`CommandRegistry::register_guild`] (bulk overwrite).
//! 3. Hand [`InteractionCreate`](axis_model::InteractionType) events to
//!    [`CommandRegistry::handle`]; it resolves the command (and
//!    subcommand path), invokes the handler, and answers autocomplete
//!    requests.

pub mod invocation;
pub mod registry;

pub use invocation::{FromCommandValue, Invocation, ReplyBuilder};
pub use registry::{
    CommandBuilder, CommandOption, CommandRegistry, GroupBuilder, OptionBuilder, SubCommandBuilder,
};
