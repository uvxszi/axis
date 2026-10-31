//! Internal route representation for rate-limit bucketing.

use reqwest::Method;

/// A REST route: the concrete request path plus the identity used for
/// rate-limit bucketing.
///
/// Discord computes limits per (bucket hash, major parameter); the bucket
/// hash is learned from response headers. `template` identifies the route
/// shape until the hash is known, and `major` carries the major parameter
/// (a channel, guild, or webhook ID) that splits otherwise-shared limits.
#[derive(Debug, Clone)]
pub(crate) struct Route {
    /// The HTTP method.
    pub method: Method,
    /// Path template with placeholders, e.g. `/channels/{channel_id}/messages`.
    pub template: &'static str,
    /// Concrete request path, e.g. `/channels/123/messages`.
    pub path: String,
    /// The major parameter, when the route has one.
    pub major: Option<u64>,
}

impl Route {
    /// Build a route without a major parameter.
    // Unused on the lib path for now: every Phase 2 endpoint has a major
    // parameter. Kept for upcoming no-major endpoints (e.g. `GET /users/@me`).
    #[allow(dead_code)]
    pub(crate) fn new(method: Method, template: &'static str, path: String) -> Self {
        Self {
            method,
            template,
            path,
            major: None,
        }
    }

    /// Build a route whose limit is split by a major parameter.
    pub(crate) fn with_major(
        method: Method,
        template: &'static str,
        path: String,
        major: u64,
    ) -> Self {
        Self {
            method,
            template,
            path,
            major: Some(major),
        }
    }

    /// Key identifying the route shape, used to learn bucket hashes.
    pub(crate) fn route_key(&self) -> String {
        format!("{} {}", self.method, self.template)
    }

    /// Key identifying the (bucket hash, major parameter) pair that
    /// Discord's per-route limits are keyed by.
    pub(crate) fn bucket_key(&self, hash: &str) -> String {
        match self.major {
            Some(major) => format!("{hash}#{major}"),
            None => format!("{hash}#"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_key_identifies_shape() {
        let a = Route::with_major(
            Method::POST,
            "/channels/{channel_id}/messages",
            "/channels/1/messages".to_owned(),
            1,
        );
        let b = Route::with_major(
            Method::POST,
            "/channels/{channel_id}/messages",
            "/channels/2/messages".to_owned(),
            2,
        );
        let c = Route::new(
            Method::GET,
            "/channels/{channel_id}/messages",
            "/channels/1/messages".to_owned(),
        );
        // Same shape, different methods: different limits.
        assert_ne!(a.route_key(), c.route_key());
        // Same shape and method, different major parameters: same route
        // key (the bucket hash is shared), different bucket keys.
        assert_eq!(a.route_key(), b.route_key());
        assert_ne!(a.bucket_key("abc"), b.bucket_key("abc"));
        assert_eq!(a.bucket_key("abc"), "abc#1");
    }

    #[test]
    fn no_major_parameter() {
        let route = Route::new(Method::GET, "/gateway", "/gateway".to_owned());
        assert_eq!(route.bucket_key("abc"), "abc#");
    }
}
