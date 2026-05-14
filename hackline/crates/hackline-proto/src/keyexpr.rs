//! Key-expression builders. The catalogue is in `DOCS/KEYEXPRS.md`;
//! every keyexpr in that table is built by exactly one function in
//! this file so a typo can't slip into the wire.

use uuid::Uuid;

use crate::zid::Zid;

/// `hackline/<zid>/tcp/<port>/connect`
pub fn connect(zid: &Zid, port: u16) -> String {
    format!("hackline/{}/tcp/{}/connect", zid, port)
}

/// `hackline/<zid>/info`
pub fn info(zid: &Zid) -> String {
    format!("hackline/{}/info", zid)
}

/// `hackline/<zid>/health`
pub fn health(zid: &Zid) -> String {
    format!("hackline/{}/health", zid)
}

/// `hackline/<zid>/stream/<request_id>/gw` — gateway → agent data.
pub fn stream_gw(zid: &Zid, request_id: &Uuid) -> String {
    format!("hackline/{}/stream/{}/gw", zid, request_id)
}

/// `hackline/<zid>/stream/<request_id>/dev` — agent → gateway data.
pub fn stream_dev(zid: &Zid, request_id: &Uuid) -> String {
    format!("hackline/{}/stream/{}/dev", zid, request_id)
}

/// Dotted topic → keyexpr suffix. `graph.slot.temp.changed` →
/// `graph/slot/temp/changed`. SCOPE.md §5.5 forbids `.` inside a
/// topic segment; callers validate before publishing.
pub fn topic_to_keyexpr_suffix(topic: &str) -> String {
    topic.replace('.', "/")
}

/// `hackline/<zid>/msg/event/<topic-as-keyexpr>`
pub fn msg_event(zid: &Zid, topic: &str) -> String {
    format!(
        "hackline/{}/msg/event/{}",
        zid,
        topic_to_keyexpr_suffix(topic)
    )
}

/// `hackline/<zid>/msg/log/<topic-as-keyexpr>`
pub fn msg_log(zid: &Zid, topic: &str) -> String {
    format!(
        "hackline/{}/msg/log/{}",
        zid,
        topic_to_keyexpr_suffix(topic)
    )
}

/// Gateway-side fan-in subscription for every device's events.
pub const MSG_EVENT_FANIN: &str = "hackline/*/msg/event/**";

/// Gateway-side fan-in subscription for every device's logs.
pub const MSG_LOG_FANIN: &str = "hackline/*/msg/log/**";

/// Inbound message-plane keyexpr kinds the gateway recognises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgKind {
    Event,
    Log,
}

/// Parse a concrete inbound key like
/// `hackline/<zid>/msg/event/foo/bar` back into
/// `(zid, kind, dotted-topic)`. Returns `None` if the shape doesn't
/// match — callers log-and-drop rather than crash on a malformed
/// publication from an untrusted device.
pub fn parse_msg_keyexpr(ke: &str) -> Option<(Zid, MsgKind, String)> {
    let mut parts = ke.split('/');
    if parts.next()? != "hackline" {
        return None;
    }
    let zid_raw = parts.next()?;
    if parts.next()? != "msg" {
        return None;
    }
    let kind = match parts.next()? {
        "event" => MsgKind::Event,
        "log" => MsgKind::Log,
        _ => return None,
    };
    let rest: Vec<&str> = parts.collect();
    if rest.is_empty() {
        return None;
    }
    let zid = Zid::new(zid_raw).ok()?;
    let topic = rest.join(".");
    Some((zid, kind, topic))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyexpr_shape() {
        let zid = Zid::new("aabb").unwrap();
        assert_eq!(connect(&zid, 22), "hackline/aabb/tcp/22/connect");
        assert_eq!(info(&zid), "hackline/aabb/info");
        assert_eq!(health(&zid), "hackline/aabb/health");

        let rid = Uuid::nil();
        assert_eq!(
            stream_gw(&zid, &rid),
            "hackline/aabb/stream/00000000-0000-0000-0000-000000000000/gw"
        );
    }

    #[test]
    fn msg_keyexpr_round_trip() {
        let zid = Zid::new("aabb").unwrap();
        let ke = msg_event(&zid, "graph.slot.temp.changed");
        assert_eq!(ke, "hackline/aabb/msg/event/graph/slot/temp/changed");

        let (z, kind, topic) = parse_msg_keyexpr(&ke).unwrap();
        assert_eq!(z.as_str(), "aabb");
        assert_eq!(kind, MsgKind::Event);
        assert_eq!(topic, "graph.slot.temp.changed");

        let log_ke = msg_log(&zid, "audit.entry");
        let (_, kind, topic) = parse_msg_keyexpr(&log_ke).unwrap();
        assert_eq!(kind, MsgKind::Log);
        assert_eq!(topic, "audit.entry");
    }

    #[test]
    fn parse_rejects_bad_shapes() {
        assert!(parse_msg_keyexpr("hackline/aabb/msg/event").is_none());
        assert!(parse_msg_keyexpr("hackline/aabb/info").is_none());
        assert!(parse_msg_keyexpr("hackline/ZZ/msg/event/x").is_none());
        assert!(parse_msg_keyexpr("hackline/aabb/msg/cmd/x").is_none());
    }
}
