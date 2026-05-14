//! `devices` table repository: insert, list, get, delete, scoped by
//! `org_id`. SCOPE.md §13 Phase 4: every device belongs to exactly
//! one org; cross-org reads return `NotFound` (handlers translate to
//! 404, never 403, so cross-org probing cannot enumerate ids).

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::error::GatewayError;

#[derive(Debug, Serialize)]
pub struct Device {
    pub id: i64,
    pub org_id: i64,
    pub zid: String,
    pub label: String,
    pub customer_id: Option<i64>,
    pub created_at: i64,
    pub last_seen_at: Option<i64>,
}

pub fn insert(
    conn: &Connection,
    org_id: i64,
    zid: &str,
    label: &str,
) -> Result<Device, GatewayError> {
    conn.execute(
        "INSERT INTO devices (org_id, zid, label, created_at) VALUES (?1, ?2, ?3, unixepoch())",
        params![org_id, zid, label],
    )?;
    let id = conn.last_insert_rowid();
    get_in_org(conn, org_id, id)
}

pub fn list_in_org(conn: &Connection, org_id: i64) -> Result<Vec<Device>, GatewayError> {
    let mut stmt = conn.prepare(
        "SELECT id, org_id, zid, label, customer_id, created_at, last_seen_at
         FROM devices WHERE org_id = ?1 ORDER BY id",
    )?;
    let rows = stmt.query_map(params![org_id], row_to_device)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// Fetch a device by id within the caller's org. Returns `NotFound`
/// for both "no such id" and "id belongs to another org" — the
/// caller never gets a way to distinguish the two, which means an
/// attacker can't enumerate cross-org ids by 403-vs-404 timing
/// (SCOPE.md §13 Phase 4 design note).
pub fn get_in_org(conn: &Connection, org_id: i64, id: i64) -> Result<Device, GatewayError> {
    conn.query_row(
        "SELECT id, org_id, zid, label, customer_id, created_at, last_seen_at
         FROM devices WHERE id = ?1 AND org_id = ?2",
        params![id, org_id],
        row_to_device,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GatewayError::NotFound,
        other => GatewayError::Db(other),
    })
}

pub fn delete_in_org(conn: &Connection, org_id: i64, id: i64) -> Result<bool, GatewayError> {
    let n = conn.execute(
        "DELETE FROM devices WHERE id = ?1 AND org_id = ?2",
        params![id, org_id],
    )?;
    Ok(n > 0)
}

/// Resolve a device id to `(Device, org_slug)`. Used by background
/// loops (cmd delivery, api_call) that need the org slug to build
/// the right keyexpr for the device's tenant.
pub fn get_with_org_slug(
    conn: &Connection,
    id: i64,
) -> Result<(Device, String), GatewayError> {
    conn.query_row(
        "SELECT d.id, d.org_id, d.zid, d.label, d.customer_id, d.created_at, d.last_seen_at,
                o.slug
         FROM devices d JOIN orgs o ON o.id = d.org_id
         WHERE d.id = ?1",
        params![id],
        |row| {
            Ok((
                Device {
                    id: row.get(0)?,
                    org_id: row.get(1)?,
                    zid: row.get(2)?,
                    label: row.get(3)?,
                    customer_id: row.get(4)?,
                    created_at: row.get(5)?,
                    last_seen_at: row.get(6)?,
                },
                row.get::<_, String>(7)?,
            ))
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GatewayError::NotFound,
        other => GatewayError::Db(other),
    })
}

/// Cross-org lookup used by background loops (cmd delivery, fan-in
/// attribution) that have already gone through the per-org REST
/// gate when the row was created. Not exposed to handlers.
pub fn get_unscoped(conn: &Connection, id: i64) -> Result<Device, GatewayError> {
    conn.query_row(
        "SELECT id, org_id, zid, label, customer_id, created_at, last_seen_at
         FROM devices WHERE id = ?1",
        params![id],
        row_to_device,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GatewayError::NotFound,
        other => GatewayError::Db(other),
    })
}

/// Cross-org lookup used by Zenoh fan-in paths that have a `zid` and
/// need to attribute an inbound message to the owning device.
/// Returns `(device_id, org_id)` so the caller can use the org id
/// for downstream queries; the wire frame's keyexpr must include the
/// org slug too — see SCOPE.md §5.1 and the `org_slug` plumbing in
/// `msg_fanin`.
pub fn get_by_zid(conn: &Connection, zid: &str) -> Result<Option<(i64, i64)>, GatewayError> {
    use rusqlite::OptionalExtension;
    conn.query_row(
        "SELECT id, org_id FROM devices WHERE zid = ?1",
        params![zid],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    )
    .optional()
    .map_err(GatewayError::Db)
}

fn row_to_device(row: &rusqlite::Row) -> rusqlite::Result<Device> {
    Ok(Device {
        id: row.get(0)?,
        org_id: row.get(1)?,
        zid: row.get(2)?,
        label: row.get(3)?,
        customer_id: row.get(4)?,
        created_at: row.get(5)?,
        last_seen_at: row.get(6)?,
    })
}
