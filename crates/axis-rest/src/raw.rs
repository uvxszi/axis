//! Raw access to any REST endpoint, for everything the typed builders
//! don't cover yet. Requests still go through the rate limiter, retries,
//! and audit-log reasons.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use axis_core::{AxisError, ConnectionError, InvalidDataError, Result, SerializationError};
use reqwest::Method;
use serde_json::Value;

use crate::{client::Http, routing::Route};

/// Route shapes are interned so rate-limit buckets can be learned per
/// shape (`/channels/{id}/messages`), not per concrete path.
fn intern(template: String) -> &'static str {
    static TEMPLATES: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    let mut templates = TEMPLATES
        .get_or_init(Default::default)
        .lock()
        .expect("templates poisoned");
    if let Some(existing) = templates.get(template.as_str()) {
        return existing;
    }
    let leaked: &'static str = Box::leak(template.into_boxed_str());
    templates.insert(leaked);
    leaked
}

/// `/channels/123/messages/456` → (`/channels/{id}/messages/{id}`, 123).
fn shape(path: &str) -> (String, Option<u64>) {
    let mut major = None;
    let mut previous = "";
    let template = path
        .split('/')
        .map(|segment| {
            let is_id = !segment.is_empty() && segment.chars().all(|c| c.is_ascii_digit());
            let out = if is_id {
                if major.is_none() && matches!(previous, "channels" | "guilds" | "webhooks") {
                    major = segment.parse().ok();
                }
                "{id}"
            } else {
                segment
            };
            previous = segment;
            out.to_owned()
        })
        .collect::<Vec<_>>()
        .join("/");
    (template, major)
}

impl Http {
    /// Call any Discord REST endpoint. `path` is relative to the API base
    /// (e.g. `/channels/123/voice-status`); `reason` goes to the audit log.
    /// Returns the response JSON, or `Value::Null` for empty responses.
    pub async fn raw(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
        reason: Option<String>,
    ) -> Result<Value> {
        let method = Method::from_bytes(method.trim().to_uppercase().as_bytes()).map_err(|_| {
            AxisError::InvalidData(InvalidDataError::new(format!("bad HTTP method {method}")))
        })?;
        let path = path.trim();
        let path = path
            .trim_start_matches("https://discord.com")
            .trim_start_matches("/api");
        let path = format!("/{}", path.trim_start_matches('/'));
        let path = path
            .strip_prefix("/v10")
            .or_else(|| path.strip_prefix("/v9"))
            .map(str::to_owned)
            .unwrap_or(path);
        let (template, major) = shape(path.split('?').next().unwrap_or(&path));
        let template = intern(format!("RAW {template}"));
        let route = match major {
            Some(major) => Route::with_major(method, template, path.clone(), major),
            None => Route::new(method, template, path.clone()),
        }
        .with_reason(reason);
        let body = body
            .map(|b| serde_json::to_vec(&b))
            .transpose()
            .map_err(|error| AxisError::Serialization(SerializationError::Json(error)))?;
        let response = self.send(route, body, None).await?;
        let bytes = response.bytes().await.map_err(|error| {
            AxisError::Connection(ConnectionError::with_source(
                "failed to read response body",
                error,
            ))
        })?;
        if bytes.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_slice(&bytes)
            .map_err(|error| AxisError::Serialization(SerializationError::Json(error)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_paths_into_buckets() {
        assert_eq!(
            shape("/channels/123/messages/456"),
            ("/channels/{id}/messages/{id}".to_owned(), Some(123))
        );
        assert_eq!(
            shape("/guilds/9/members/5"),
            ("/guilds/{id}/members/{id}".to_owned(), Some(9))
        );
        assert_eq!(shape("/users/@me"), ("/users/@me".to_owned(), None));
        assert!(std::ptr::eq(intern("x".into()), intern("x".into())));
    }
}
