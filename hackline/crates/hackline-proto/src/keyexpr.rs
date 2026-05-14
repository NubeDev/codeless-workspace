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
}
