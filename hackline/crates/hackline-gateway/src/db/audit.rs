//! `audit` table repository. Two row shapes share the table:
//!
//! - **Point-in-time actions** (`cmd.send`, `api.call`, `device.create`,
//!   ...): `insert(...)` writes one row with `ts` + `action` + `detail`.
//! - **Tunnel sessions** (`tunnel.session`): `insert_tunnel_session_open`
//!   writes one row at bridge open and `finalize_tunnel_session`
//!   updates the same row with byte counters and `ts_close` when the
//!   bridge closes. Per-event logging would be hundreds of millions of
//!   rows/year at fleet scale, so a session is one row, not two.
//!
//! Retention strategy is documented in `DOCS/DATABASE.md`.

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::error::GatewayError;

#[derive(Debug, Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub ts: i64,
    pub ts_close: Option<i64>,
    pub user_id: Option<i64>,
    pub device_id: Option<i64>,
    pub tunnel_id: Option<i64>,
    pub request_id: Option<String>,
    pub action: String,
    pub peer: Option<String>,
    pub bytes_up: Option<i64>,
    pub bytes_down: Option<i64>,
    pub detail: Option<String>,
}

/// Append a point-in-time audit action. Used by every handler that
/// emits a SCOPE.md §7.2 well-known action other than `tunnel.session`.
pub fn insert(
    conn: &Connection,
    user_id: Option<i64>,
    device_id: Option<i64>,
    tunnel_id: Option<i64>,
    action: &str,
    detail: Option<&str>,
) -> Result<(), GatewayError> {
    conn.execute(
        "INSERT INTO audit (ts, user_id, device_id, tunnel_id, action, detail)
         VALUES (unixepoch(), ?1, ?2, ?3, ?4, ?5)",
        params![user_id, device_id, tunnel_id, action, detail],
    )?;
    Ok(())
}

/// Insert the `tunnel.session` row at bridge open. Returns the row id
/// so the caller can finalise it with byte counters when the bridge
/// closes.
pub fn insert_tunnel_session_open(
    conn: &Connection,
    device_id: i64,
    tunnel_id: Option<i64>,
    request_id: &str,
    peer: Option<&str>,
    ts_open_ms: i64,
) -> Result<i64, GatewayError> {
    conn.execute(
        "INSERT INTO audit (ts, user_id, device_id, tunnel_id, action, request_id, peer)
         VALUES (?1, NULL, ?2, ?3, 'tunnel.session', ?4, ?5)",
        params![ts_open_ms, device_id, tunnel_id, request_id, peer],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Stamp the close-time fields on a `tunnel.session` row.
pub fn finalize_tunnel_session(
    conn: &Connection,
    id: i64,
    bytes_up: u64,
    bytes_down: u64,
    ts_close_ms: i64,
) -> Result<(), GatewayError> {
    conn.execute(
        "UPDATE audit
         SET ts_close = ?2, bytes_up = ?3, bytes_down = ?4
         WHERE id = ?1",
        params![id, ts_close_ms, bytes_up as i64, bytes_down as i64],
    )?;
    Ok(())
}

/// Recent audit rows visible to `org_id`. SCOPE.md §13 Phase 4:
/// a row is visible iff its `device_id` lives in the caller's org,
/// or it has no device id (claim, auth.login) and its `user_id`
/// belongs to the caller's org. Rows referencing a device the caller
/// doesn't own (or a user in another org) are filtered out at the
/// SQL level so a cross-org caller cannot even count them.
pub fn list_recent(
    conn: &Connection,
    org_id: i64,
    limit: i64,
) -> Result<Vec<AuditEntry>, GatewayError> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.ts, a.ts_close, a.user_id, a.device_id, a.tunnel_id,
                a.request_id, a.action, a.peer, a.bytes_up, a.bytes_down, a.detail
         FROM audit a
         LEFT JOIN devices d ON d.id = a.device_id
         LEFT JOIN users   u ON u.id = a.user_id
         WHERE (a.device_id IS NULL OR d.org_id = ?1)
           AND (a.user_id   IS NULL OR u.org_id = ?1)
         ORDER BY a.id DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![org_id, limit], |row| {
        Ok(AuditEntry {
            id: row.get(0)?,
            ts: row.get(1)?,
            ts_close: row.get(2)?,
            user_id: row.get(3)?,
            device_id: row.get(4)?,
            tunnel_id: row.get(5)?,
            request_id: row.get(6)?,
            action: row.get(7)?,
            peer: row.get(8)?,
            bytes_up: row.get(9)?,
            bytes_down: row.get(10)?,
            detail: row.get(11)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn count(conn: &Connection) -> Result<i64, GatewayError> {
    conn.query_row("SELECT COUNT(*) FROM audit", [], |row| row.get::<_, i64>(0))
        .map_err(GatewayError::Db)
}
