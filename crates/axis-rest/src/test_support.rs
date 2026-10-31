//! Shared helpers for unit tests inside the crate.

use axis_core::Token;

use crate::client::Http;

/// A client for tests that only exercise local validation and therefore
/// never perform a real request. Points at a port where nothing listens.
pub(crate) fn http_for_tests() -> Http {
    Http::builder(Token::new("test-token".to_owned()))
        .api_base("http://127.0.0.1:9")
        .build()
        .expect("test client builds")
}
