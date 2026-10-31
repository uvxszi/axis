//! Optional default logging setup.
//!
//! Axis logs through the [`tracing`] facade and never installs a subscriber
//! on its own — the application decides how logs are collected. These
//! helpers install a sensible default subscriber for applications that do
//! not want to configure one themselves.
//!
//! Applications that set up their own subscriber (or a library user that
//! must not install global state) should simply not call these functions.

use crate::error::{AxisError, InvalidDataError, Result};

/// Install a default `tracing` subscriber.
///
/// The filter is taken from the `RUST_LOG` environment variable when set
/// (using `env_filter` syntax, e.g. `RUST_LOG=axis=debug`), otherwise it
/// defaults to `info` for everything.
///
/// Returns an error if a global subscriber is already installed.
///
/// # Examples
///
/// ```no_run
/// axis_core::log::init().expect("failed to install logging");
/// ```
pub fn init() -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    install(filter)
}

/// Install a default `tracing` subscriber with an explicit filter directive
/// string, e.g. `"axis=debug,hyper=warn"`.
///
/// Returns an error if the filter cannot be parsed or if a global
/// subscriber is already installed.
pub fn init_with_filter(filter: &str) -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_new(filter).map_err(|error| {
        AxisError::InvalidData(InvalidDataError::with_source(
            format!("invalid log filter {filter:?}"),
            error,
        ))
    })?;
    install(filter)
}

fn install(filter: tracing_subscriber::EnvFilter) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init()
        .map_err(|error| {
            AxisError::InvalidData(InvalidDataError::with_source(
                "a global tracing subscriber is already installed",
                error,
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_filter_is_rejected() {
        // Deterministic regardless of whether a subscriber is installed:
        // the filter is parsed before the subscriber is touched.
        let result = init_with_filter("%%% definitely not a filter");
        assert!(
            matches!(result, Err(AxisError::InvalidData(InvalidDataError { .. }))),
            "expected a filter parse error, got {result:?}"
        );
    }
}
