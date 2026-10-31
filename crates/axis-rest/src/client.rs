//! The REST client: authentication, the request pipeline, and error
//! mapping.

use std::time::Duration;

use reqwest::{
    Method, StatusCode,
    header::{AUTHORIZATION, CONTENT_TYPE},
};
use serde::de::DeserializeOwned;
use tracing::{debug, warn};

use axis_core::{
    AuthError, ConnectionError, HttpError, InvalidDataError, RateLimitError, SerializationError,
    Token,
    error::{AxisError, Result},
};

use crate::{ratelimit::RateLimiter, routing::Route};

/// The default Discord API base URL (version 10).
pub const API_BASE: &str = "https://discord.com/api/v10";

/// Asynchronous client for the Discord REST API.
///
/// All requests go through one pipeline that
///
/// - authenticates with the configured bot token (never logged or leaked),
/// - respects per-route and global rate limits, waiting for bucket resets
///   and honoring Discord's `retry_after` guidance,
/// - retries transient failures (network errors and 5xx) a bounded number
///   of times and only for idempotent methods,
/// - maps every non-success response to a typed [`AxisError`] with the
///   status, route, and Discord error code/message.
///
/// Endpoints are exposed as methods returning awaitable builders, e.g.
/// [`Http::create_message`] → `CreateMessage` → `Message`.
#[derive(Debug)]
pub struct Http {
    auth_header: String,
    api_base: String,
    client: reqwest::Client,
    limiter: RateLimiter,
    max_retries: u32,
    max_rate_limit_retries: u32,
}

impl Http {
    /// Create a builder for an `Http` client.
    ///
    /// The token is wrapped in a redacting [`Token`]; pass one built with
    /// [`Token::from_env`](axis_core::Token::from_env) or your own secret
    /// store.
    pub fn builder(token: Token) -> HttpBuilder {
        HttpBuilder::new(token)
    }

    /// Send a request through the full pipeline and return the 2xx
    /// response.
    pub(crate) async fn send(
        &self,
        route: Route,
        body: Option<Vec<u8>>,
        query: Option<&[(&'static str, String)]>,
    ) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.api_base, route.path);
        let mut rate_limit_attempts: u32 = 0;
        let mut transient_attempts: u32 = 0;

        loop {
            self.limiter.acquire(&route).await;

            let mut request = self
                .client
                .request(route.method.clone(), &url)
                .header(AUTHORIZATION, &self.auth_header);
            if let Some(body) = &body {
                request = request
                    .header(CONTENT_TYPE, "application/json")
                    .body(body.clone());
            }
            if let Some(query) = query {
                request = request.query(query);
            }

            debug!(method = %route.method, path = %route.path, "sending request");
            let response = match request.send().await {
                Ok(response) => response,
                Err(error) => {
                    if Self::is_idempotent(&route) && transient_attempts < self.max_retries {
                        transient_attempts += 1;
                        warn!(
                            path = %route.path,
                            attempt = transient_attempts,
                            error = %error,
                            "request failed; retrying after backoff"
                        );
                        Self::sleep_backoff(transient_attempts).await;
                        continue;
                    }
                    return Err(AxisError::Connection(ConnectionError::with_source(
                        format!("request to {} failed", route.error_route()),
                        error,
                    )));
                }
            };

            let status = response.status();
            let headers = response.headers().clone();
            self.limiter.update_from_headers(&route, &headers);

            if status.is_success() {
                return Ok(response);
            }

            if status == StatusCode::TOO_MANY_REQUESTS {
                let info = RateLimitInfo::parse(&headers, response).await;
                if info.global {
                    self.limiter.penalize_globally(info.retry_after);
                }
                if rate_limit_attempts >= self.max_rate_limit_retries {
                    return Err(AxisError::RateLimit(info.into_error()));
                }
                rate_limit_attempts += 1;
                warn!(
                    path = %route.path,
                    attempt = rate_limit_attempts,
                    wait_ms = info.retry_after.as_millis() as u64,
                    global = info.global,
                    message = ?info.message,
                    "rate limited; retrying"
                );
                tokio::time::sleep(info.retry_after).await;
                continue;
            }

            if status == StatusCode::UNAUTHORIZED {
                // Discord's guidance: an invalid token must stop requests,
                // not hammer the API. No retries.
                return Err(AxisError::Auth(AuthError::Invalid));
            }

            if status.is_server_error()
                && Self::is_idempotent(&route)
                && transient_attempts < self.max_retries
            {
                transient_attempts += 1;
                warn!(
                    path = %route.path,
                    attempt = transient_attempts,
                    status = status.as_u16(),
                    "server error; retrying after backoff"
                );
                Self::sleep_backoff(transient_attempts).await;
                continue;
            }

            let text = response.text().await.map_err(|error| {
                AxisError::Connection(ConnectionError::with_source(
                    "failed to read error response body",
                    error,
                ))
            })?;
            let (code, message) = parse_error_body(&text);
            let mut http_error = HttpError::new(status.as_u16(), route.error_route().to_owned());
            if let Some(code) = code {
                http_error = http_error.with_code(code);
            }
            if let Some(message) = message {
                http_error = http_error.with_message(message);
            }
            return Err(AxisError::Http(http_error));
        }
    }

    /// Send a request and deserialize the JSON response body.
    pub(crate) async fn json<T: DeserializeOwned>(
        &self,
        route: Route,
        body: Option<Vec<u8>>,
        query: Option<&[(&'static str, String)]>,
    ) -> Result<T> {
        let response = self.send(route, body, query).await?;
        let bytes = response.bytes().await.map_err(|error| {
            AxisError::Connection(ConnectionError::with_source(
                "failed to read response body",
                error,
            ))
        })?;
        serde_json::from_slice(&bytes)
            .map_err(|error| AxisError::Serialization(SerializationError::Json(error)))
    }

    /// Send a request whose success response has no meaningful body
    /// (typically HTTP 204).
    pub(crate) async fn empty(&self, route: Route, body: Option<Vec<u8>>) -> Result<()> {
        let response = self.send(route, body, None).await?;
        // Drain the body so the connection can be reused.
        response.bytes().await.map_err(|error| {
            AxisError::Connection(ConnectionError::with_source(
                "failed to read response body",
                error,
            ))
        })?;
        Ok(())
    }

    fn is_idempotent(route: &Route) -> bool {
        matches!(route.method, Method::GET | Method::PUT | Method::DELETE)
    }

    /// Exponential backoff between transient retries: 500ms, 1s, 2s, ...,
    /// capped at 5s.
    async fn sleep_backoff(attempt: u32) {
        let shift = attempt.saturating_sub(1).min(4);
        let millis = 500_u64.saturating_mul(1 << shift).min(5_000);
        tokio::time::sleep(Duration::from_millis(millis)).await;
    }
}

/// Parse Discord's error body shape `{"code": 50035, "message": "..."}`,
/// tolerating non-JSON bodies.
fn parse_error_body(text: &str) -> (Option<u32>, Option<String>) {
    #[derive(serde::Deserialize)]
    struct ErrorBody {
        code: Option<u32>,
        message: Option<String>,
    }

    serde_json::from_str::<ErrorBody>(text)
        .map(|body| (body.code, body.message))
        .unwrap_or((None, None))
}

/// The parsed contents of a 429 response.
#[derive(Debug)]
struct RateLimitInfo {
    retry_after: Duration,
    global: bool,
    message: Option<String>,
    bucket: Option<String>,
    scope: Option<String>,
}

impl RateLimitInfo {
    /// Parse a 429 response. The body's `retry_after` is authoritative per
    /// Discord; the `Retry-After` header and a conservative 1s default are
    /// the fallbacks (Cloudflare blocks return HTML, not JSON).
    async fn parse(headers: &reqwest::header::HeaderMap, response: reqwest::Response) -> Self {
        #[derive(serde::Deserialize)]
        struct RateLimitBody {
            message: Option<String>,
            retry_after: Option<f64>,
            global: Option<bool>,
        }

        let text = response.text().await.unwrap_or_default();
        let body = serde_json::from_str::<RateLimitBody>(&text).ok();

        let retry_after = body
            .as_ref()
            .and_then(|body| body.retry_after)
            .or_else(|| header_f64(headers, "retry-after"))
            .filter(|value| value.is_finite() && *value >= 0.0)
            .unwrap_or(1.0);
        let retry_after =
            Duration::try_from_secs_f64(retry_after).unwrap_or(Duration::from_secs(1));

        let global = body
            .as_ref()
            .and_then(|body| body.global)
            .unwrap_or_else(|| {
                headers
                    .get("x-ratelimit-global")
                    .and_then(|value| value.to_str().ok())
                    .is_some_and(|value| value.eq_ignore_ascii_case("true"))
            });

        Self {
            retry_after,
            global,
            message: body.and_then(|body| body.message),
            bucket: header_str(headers, "x-ratelimit-bucket").map(str::to_owned),
            scope: header_str(headers, "x-ratelimit-scope").map(str::to_owned),
        }
    }

    fn into_error(self) -> RateLimitError {
        RateLimitError {
            global: self.global,
            retry_after: self.retry_after,
            bucket: self.bucket,
            scope: self.scope,
        }
    }
}

/// Builder for [`Http`].
#[derive(Debug)]
pub struct HttpBuilder {
    token: Token,
    api_base: String,
    timeout: Option<Duration>,
    user_agent: String,
    max_retries: u32,
    max_rate_limit_retries: u32,
}

impl HttpBuilder {
    /// Create a builder with defaults: API v10, a 30s request timeout, the
    /// `Axis/<version>` user agent, and up to 3 transient retries and 3
    /// rate-limit retries.
    pub fn new(token: Token) -> Self {
        Self {
            token,
            api_base: API_BASE.to_owned(),
            timeout: Some(Duration::from_secs(30)),
            user_agent: format!("Axis/{}", env!("CARGO_PKG_VERSION")),
            max_retries: 3,
            max_rate_limit_retries: 3,
        }
    }

    /// Override the API base URL — for tests against a mock server or for
    /// forward compatibility.
    pub fn api_base(mut self, api_base: impl Into<String>) -> Self {
        self.api_base = api_base.into();
        self
    }

    /// Set the per-request timeout. Requests spend this budget on the HTTP
    /// call itself, not on rate-limit waits.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Remove the per-request timeout.
    pub fn no_timeout(mut self) -> Self {
        self.timeout = None;
        self
    }

    /// Override the user agent Discord sees.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// How many times transient failures (network errors and 5xx on
    /// idempotent methods) may be retried before surfacing an error.
    pub fn max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// How many 429 responses may be waited out and retried before
    /// surfacing a [`RateLimitError`].
    pub fn max_rate_limit_retries(mut self, max: u32) -> Self {
        self.max_rate_limit_retries = max;
        self
    }

    /// Build the client. Never contacts Discord.
    pub fn build(self) -> Result<Http> {
        // Normalize the token: users pass either the raw token or one
        // already prefixed with "Bot ". The prefix is re-added exactly
        // once here; nothing logs the value.
        let raw = self.token.into_inner();
        let raw = raw.strip_prefix("Bot ").unwrap_or(&raw).trim().to_owned();
        if raw.is_empty() {
            return Err(AxisError::Auth(AuthError::Missing));
        }

        let mut client_builder = reqwest::Client::builder().user_agent(self.user_agent);
        if let Some(timeout) = self.timeout {
            client_builder = client_builder.timeout(timeout);
        }
        let client = client_builder.build().map_err(|error| {
            AxisError::InvalidData(InvalidDataError::with_source(
                "failed to build the HTTP client",
                error,
            ))
        })?;

        Ok(Http {
            auth_header: format!("Bot {raw}"),
            api_base: self.api_base.trim_end_matches('/').to_owned(),
            client,
            limiter: RateLimiter::new(),
            max_retries: self.max_retries,
            max_rate_limit_retries: self.max_rate_limit_retries,
        })
    }
}

fn header_str<'a>(headers: &'a reqwest::header::HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok()
}

fn header_f64(headers: &reqwest::header::HeaderMap, name: &str) -> Option<f64> {
    header_str(headers, name)?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_body_parsing() {
        assert_eq!(
            parse_error_body(r#"{"code": 50035, "message": "Invalid Form Body"}"#),
            (Some(50_035), Some("Invalid Form Body".to_owned()))
        );
        // Non-JSON (e.g. Cloudflare HTML) yields no code or message.
        assert_eq!(parse_error_body("<html>blocked</html>"), (None, None));
        assert_eq!(parse_error_body(""), (None, None));
    }

    #[test]
    fn builder_rejects_empty_token() {
        let result = Http::builder(Token::new("   ".to_owned())).build();
        assert!(matches!(result, Err(AxisError::Auth(AuthError::Missing))));
    }

    #[test]
    fn idempotent_classification() {
        let get = Route::new(Method::GET, "/x", "/x".to_owned());
        let post = Route::new(Method::POST, "/x", "/x".to_owned());
        assert!(Http::is_idempotent(&get));
        assert!(!Http::is_idempotent(&post));
    }
}
