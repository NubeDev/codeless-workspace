//! `audit` table repository: append + cursor-paginated read.
//! Retention strategy is documented in `DOCS/DATABASE.md`.

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::error::GatewayError;

#[derive(Debug, Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub ts: i64,
    pub user_id: Option<i64>,
    pub device_id: Option<i64>,
    pub tunnel_id: Option<i64>,
    pub action: String,
    pub detail: Option<String>,
}

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

pub fn list_recent(conn: &Connection, limit: i64) -> Result<Vec<AuditEntry>, GatewayError> {
    let mut stmt = conn.prepare(
        "SELECT id, ts, user_id, device_id, tunnel_id, action, detail
         FROM audit ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        Ok(AuditEntry {
            id: row.get(0)?,
            ts: row.get(1)?,
            user_id: row.get(2)?,
            device_id: row.get(3)?,
            tunnel_id: row.get(4)?,
            action: row.get(5)?,
            detail: row.get(6)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}
