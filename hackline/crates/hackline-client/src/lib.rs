//! Device-side SDK. A thin wrapper around `zenoh::Session` that
//! enforces hackline's message-plane conventions: validated topics,
//! `MsgEnvelope` JSON over Zenoh, keyexprs scoped to the session's
//! own ZID. Phase 1.5 ships `publish_event` and `publish_log`;
//! `serve_api` / `subscribe_cmd` land in Phase 2.
//!
//! The SDK never opens a second auth layer — auth is Zenoh ACL on
//! the session itself (SCOPE.md §3.5 / §8.2).

use std::sync::Arc;

use hackline_proto::keyexpr;
use hackline_proto::msg::{LogLevel, MsgEnvelope};
use hackline_proto::zid::Zid;
use thiserror::Error;
use zenoh::bytes::ZBytes;
use zenoh::Session;

/// SDK error type. Surfaces config validation, Zenoh transport
/// failures, and JSON serialisation problems without forcing callers
/// to depend on Zenoh's own error type.
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("zenoh: {0}")]
    Zenoh(#[from] zenoh::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid zid: {0}")]
    Zid(String),
    #[error("invalid topic: {0}")]
    Topic(String),
}

/// SDK session. Holds a `zenoh::Session` and the device's `Zid` so
/// every published keyexpr is scoped to the session's own namespace
/// (SCOPE.md §3.5 trust model — publishing under another zid is an
/// ACL violation and would be rejected by the router anyway).
#[derive(Clone)]
pub struct ClientSession {
    inner: Arc<Session>,
    zid: Zid,
}

impl ClientSession {
    /// Wrap an already-open `zenoh::Session`. v0.1 leaves session
    /// opening to the host application (rubix-agent, tests) because
    /// the right config layer differs per consumer; the SDK adds the
    /// hackline-specific message-plane conventions on top.
    pub fn from_session(session: Arc<Session>, zid: Zid) -> Self {
        Self {
            inner: session,
            zid,
        }
    }

    /// Convenience constructor that derives the device `Zid` from the
    /// session's own Zenoh ZID. Fails if the Zenoh ZID isn't a valid
    /// `hackline_proto::Zid` (length 2..=32, lowercase hex).
    pub fn from_session_auto(session: Arc<Session>) -> Result<Self, ClientError> {
        let raw = session.zid().to_string();
        let zid = Zid::new(&raw).map_err(|e| ClientError::Zid(e.to_string()))?;
        Ok(Self::from_session(session, zid))
    }

    pub fn zid(&self) -> &Zid {
        &self.zid
    }

    /// Publish a fire-and-forget event under
    /// `hackline/<zid>/msg/event/<topic>`. Best-effort delivery: the
    /// reliable Zenoh transport guarantees in-order delivery while
    /// the link is up, but the gateway will miss anything published
    /// during an offline window (SCOPE.md §8.1).
    pub async fn publish_event(
        &self,
        topic: &str,
        payload: serde_json::Value,
    ) -> Result<(), ClientError> {
        validate_topic(topic)?;
        let env = MsgEnvelope::new_event(payload);
        let ke = keyexpr::msg_event(&self.zid, topic);
        self.publish(&ke, &env).await
    }

    /// Publish a structured log under
    /// `hackline/<zid>/msg/log/<topic>`. Same delivery semantics as
    /// `publish_event`; the gateway routes it to the `logs` table
    /// instead of `events` purely by keyexpr.
    pub async fn publish_log(
        &self,
        level: LogLevel,
        topic: &str,
        payload: serde_json::Value,
    ) -> Result<(), ClientError> {
        validate_topic(topic)?;
        let env = MsgEnvelope::new_log(level, payload);
        let ke = keyexpr::msg_log(&self.zid, topic);
        self.publish(&ke, &env).await
    }

    async fn publish(&self, ke: &str, env: &MsgEnvelope) -> Result<(), ClientError> {
        let bytes = serde_json::to_vec(env)?;
        self.inner
            .put(ke.to_owned(), ZBytes::from(bytes))
            .await
            .map_err(ClientError::Zenoh)?;
        Ok(())
    }
}

/// Reject topics that would break the keyexpr conversion or smuggle
/// wildcards on a publish path (SCOPE.md §5.5).
fn validate_topic(topic: &str) -> Result<(), ClientError> {
    if topic.is_empty() {
        return Err(ClientError::Topic("topic must not be empty".into()));
    }
    for seg in topic.split('.') {
        if seg.is_empty() {
            return Err(ClientError::Topic(format!(
                "empty segment in topic `{topic}`"
            )));
        }
        if seg.contains('/') || seg.contains('*') {
            return Err(ClientError::Topic(format!(
                "segment `{seg}` contains reserved character"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_validation() {
        assert!(validate_topic("foo").is_ok());
        assert!(validate_topic("foo.bar.baz").is_ok());
        assert!(validate_topic("").is_err());
        assert!(validate_topic(".foo").is_err());
        assert!(validate_topic("foo..bar").is_err());
        assert!(validate_topic("foo.*").is_err());
        assert!(validate_topic("foo/bar").is_err());
    }
}
