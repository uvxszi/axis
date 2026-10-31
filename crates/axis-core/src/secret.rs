//! Redaction-safe wrappers for credentials.
//!
//! Tokens and other credentials must never end up in logs, panics, or
//! serialized output. [`Secret`] wraps a value so that:
//!
//! - `Debug` prints `Secret([REDACTED])` instead of the contents, so a
//!   token accidentally embedded in an `expect`/`unwrap`/log statement is
//!   not leaked;
//! - there is deliberately no `Display` implementation, so credentials
//!   cannot be interpolated into strings by accident;
//! - there are deliberately no serde implementations, so credentials cannot
//!   be serialized into trace output or dumps by accident.
//!
//! Access to the inner value is always explicit via [`Secret::expose`] or
//! [`Secret::into_inner`].

use core::fmt;

use crate::error::{AuthError, AxisError, InvalidDataError, Result};

/// Wraps a credential so it is hidden from logs and error messages.
///
/// ```
/// use axis_core::Secret;
///
/// let secret = Secret::new("hunter2".to_owned());
/// assert_eq!(format!("{secret:?}"), "Secret([REDACTED])");
/// assert_eq!(secret.expose(), "hunter2");
/// ```
#[derive(Clone)]
pub struct Secret<T>(T);

impl<T> Secret<T> {
    /// Wrap a credential.
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    /// Explicitly access the wrapped value.
    pub fn expose(&self) -> &T {
        &self.0
    }

    /// Consume the wrapper and return the inner value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Intentionally ignores the inner value, even when `T: Debug`.
        f.write_str("Secret([REDACTED])")
    }
}

/// A Discord bot token.
///
/// Created from a string with [`Token::new`] or from the environment with
/// [`Token::from_env`]. Both deliberately avoid logging the token.
pub type Token = Secret<String>;

impl Token {
    /// Read a bot token from an environment variable.
    ///
    /// Leading and trailing whitespace is trimmed, because environment
    /// values commonly pick up stray newlines from shell profiles or
    /// `.env` files. An unset variable or an empty value is an
    /// [`AuthError::Missing`].
    pub fn from_env(var_name: &str) -> Result<Self> {
        let value = match std::env::var(var_name) {
            Ok(value) => value,
            Err(std::env::VarError::NotPresent) => return Err(AuthError::Missing.into()),
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err(AxisError::InvalidData(InvalidDataError::new(format!(
                    "environment variable {var_name:?} is not valid Unicode"
                ))));
            }
        };

        Self::from_raw_value(value)
    }

    /// Process a raw token value: trim whitespace, reject empty values.
    fn from_raw_value(value: String) -> Result<Self> {
        let value = value.trim();
        if value.is_empty() {
            return Err(AuthError::Missing.into());
        }
        Ok(Secret::new(value.to_owned()))
    }

    /// The token as a string slice, for use in `Authorization` headers.
    pub fn as_str(&self) -> &str {
        self.expose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts() {
        let secret = Secret::new("super-secret-token");
        let rendered = format!("{secret:?}");
        assert_eq!(rendered, "Secret([REDACTED])");
        assert!(!rendered.contains("super-secret-token"));
    }

    #[test]
    fn debug_redacts_without_debug_bound() {
        // A type without Debug must still be wrappable.
        struct Opaque;
        let secret = Secret::new(Opaque);
        assert_eq!(format!("{secret:?}"), "Secret([REDACTED])");
    }

    #[test]
    fn expose_and_into_inner() {
        let secret = Secret::new(42_u64);
        assert_eq!(*secret.expose(), 42);
        assert_eq!(secret.into_inner(), 42);
    }

    #[test]
    fn token_as_str() {
        let token = Token::new("Bot abc.def".to_owned());
        assert_eq!(token.as_str(), "Bot abc.def");
        assert!(!format!("{token:?}").contains("Bot abc.def"));
    }

    #[test]
    fn from_env_missing_variable_is_auth_error() {
        // A variable name that must not exist in any environment.
        let result = Token::from_env("AXIS_DEFINITELY_UNSET_TOKEN_VAR_8f2c");
        assert!(matches!(result, Err(AxisError::Auth(AuthError::Missing))));
    }

    #[test]
    fn from_raw_value_trims_and_rejects_empty() {
        let token = Token::from_raw_value("  Bot abc.def\n".to_owned()).unwrap();
        assert_eq!(token.as_str(), "Bot abc.def");

        assert!(matches!(
            Token::from_raw_value("   ".to_owned()),
            Err(AxisError::Auth(AuthError::Missing))
        ));
    }
}
