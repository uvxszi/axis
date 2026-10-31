//! Session state tracked across gateway connections.

/// The resumable state of a shard: sequence number, session id, and the
/// resume URL Discord handed out in READY.
///
/// The distinction between "reconnect" and "resume" lives here: a session
/// can only be resumed when both a `session_id` and a sequence number are
/// known; otherwise the next connection must Identify afresh.
#[derive(Debug, Default, Clone)]
pub(crate) struct Session {
    /// The last dispatch sequence number seen (`s` of op 0 payloads).
    sequence: Option<u64>,
    /// The session id from READY.
    session_id: Option<String>,
    /// The `resume_gateway_url` from READY, preferred for resume attempts.
    resume_gateway_url: Option<String>,
}

impl Session {
    /// Record a dispatch sequence number.
    pub(crate) fn observe_sequence(&mut self, sequence: u64) {
        self.sequence = Some(sequence);
    }

    /// Adopt the session identity from a READY dispatch.
    pub(crate) fn adopt_ready(&mut self, session_id: String, resume_gateway_url: Option<String>) {
        self.session_id = Some(session_id);
        if let Some(url) = resume_gateway_url {
            self.resume_gateway_url = Some(url);
        }
    }

    /// Whether a Resume attempt is possible: a session id and a sequence
    /// number are both known.
    pub(crate) fn can_resume(&self) -> bool {
        self.session_id.is_some() && self.sequence.is_some()
    }

    /// The session id, when one is held.
    pub(crate) fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// The last observed sequence number.
    pub(crate) fn sequence(&self) -> Option<u64> {
        self.sequence
    }

    /// The URL to connect to next: `resume_gateway_url` when available,
    /// otherwise the initially configured gateway URL.
    pub(crate) fn next_url<'a>(&'a self, default_url: &'a str) -> &'a str {
        self.resume_gateway_url.as_deref().unwrap_or(default_url)
    }

    /// Drop the session: the next connection must Identify afresh.
    ///
    /// The sequence number and the resume URL are cleared too: the docs
    /// direct fresh Identifies at the initially configured gateway URL,
    /// not the old session's `resume_gateway_url`.
    pub(crate) fn reset(&mut self) {
        self.session_id = None;
        self.sequence = None;
        self.resume_gateway_url = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_session_cannot_resume() {
        let session = Session::default();
        assert!(!session.can_resume());
        assert_eq!(session.sequence(), None);
    }

    #[test]
    fn ready_makes_session_resumable() {
        let mut session = Session::default();
        session.adopt_ready("abc".to_owned(), Some("wss://resume".to_owned()));
        session.observe_sequence(7);
        assert!(session.can_resume());
        assert_eq!(session.session_id(), Some("abc"));
        assert_eq!(session.sequence(), Some(7));
        assert_eq!(session.next_url("wss://default"), "wss://resume");
    }

    #[test]
    fn reset_forces_fresh_identify() {
        let mut session = Session::default();
        session.adopt_ready("abc".to_owned(), Some("wss://resume".to_owned()));
        session.observe_sequence(7);
        session.reset();
        assert!(!session.can_resume());
        assert_eq!(session.sequence(), None);
        // A fresh identify goes to the configured gateway URL, per the
        // docs — the old session's resume URL is stale.
        assert_eq!(session.next_url("wss://default"), "wss://default");
    }

    #[test]
    fn ready_without_resume_url_keeps_previous() {
        let mut session = Session::default();
        session.adopt_ready("abc".to_owned(), None);
        assert_eq!(session.next_url("wss://default"), "wss://default");
    }
}
