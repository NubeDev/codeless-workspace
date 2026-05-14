//! Message-plane wire types — what flows on
//! `hackline/<zid>/msg/event/...` and `hackline/<zid>/msg/log/...`.
//!
//! Per SCOPE.md §5.2 the envelope carries a sender-generated id, a
//! timestamp, a content type, a small headers map (trace ids, log
//! level), and an opaque payload. v0.1 encodes the whole envelope as
//! JSON over Zenoh (debuggability wins, payloads are small). The
//! `content_type` is reserved so bincode can swap in later without
//! re-versioning the keyexpr namespace.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const CONTENT_TYPE_JSON: &str = "application/json";

/// Common envelope for events and logs. The `payload` is opaque to
/// the gateway — stored as a JSON value blob in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgEnvelope {
    pub id: Uuid,
    /// Unix milliseconds since epoch.
    pub ts: i64,
    #[serde(default = "default_content_type")]
    pub content_type: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub payload: serde_json::Value,
}

fn default_content_type() -> String {
    CONTENT_TYPE_JSON.into()
}

/// Reserved header key carrying the log level on
/// `hackline/<zid>/msg/log/...` envelopes. Events do not set it.
pub const HEADER_LOG_LEVEL: &str = "level";

/// Five-level log severity. Lowercase string on the wire and in DB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "trace" => Some(Self::Trace),
            "debug" => Some(Self::Debug),
            "info" => Some(Self::Info),
            "warn" => Some(Self::Warn),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

impl MsgEnvelope {
    /// Build a fresh event envelope. Callers set the payload; id, ts,
    /// content_type are filled in.
    pub fn new_event(payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            ts: now_ms(),
            content_type: CONTENT_TYPE_JSON.into(),
            headers: BTreeMap::new(),
            payload,
        }
    }

    /// Build a fresh log envelope. Level is stored in `headers.level`
    /// so the same envelope shape works for both planes.
    pub fn new_log(level: LogLevel, payload: serde_json::Value) -> Self {
        let mut e = Self::new_event(payload);
        e.headers
            .insert(HEADER_LOG_LEVEL.into(), level.as_str().into());
        e
    }

    /// Extract the log level from headers (info if missing or unknown).
    pub fn log_level(&self) -> LogLevel {
        self.headers
            .get(HEADER_LOG_LEVEL)
            .and_then(|s| LogLevel::parse(s))
            .unwrap_or_default()
    }
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
