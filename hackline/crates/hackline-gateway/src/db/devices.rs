//! `devices` table repository: insert, list, get, patch, delete.

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::error::GatewayError;

#[derive(Debug, Serialize)]
pub struct Device {
    pub id: i64,
    pub zid: String,
    pub label: String,
    pub customer_id: Option<i64>,
    pub created_at: i64,
    pub last_seen_at: Option<i64>,
}

pub fn insert(conn: &Connection, zid: &str, label: &str) -> Result<Device, GatewayError> {
    conn.execute(
        "INSERT INTO devices (zid, label, created_at) VALUES (?1, ?2, unixepoch())",
        params![zid, label],
    )?;
    let id = conn.last_insert_rowid();
    get(conn, id)
}

pub fn list(conn: &Connection) -> Result<Vec<Device>, GatewayError> {
    let mut stmt = conn.prepare(
        "SELECT id, zid, label, customer_id, created_at, last_seen_at FROM devices ORDER BY id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Device {
            id: row.get(0)?,
            zid: row.get(1)?,
            label: row.get(2)?,
            customer_id: row.get(3)?,
            created_at: row.get(4)?,
            last_seen_at: row.get(5)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn get(conn: &Connection, id: i64) -> Result<Device, GatewayError> {
    conn.query_row(
        "SELECT id, zid, label, customer_id, created_at, last_seen_at FROM devices WHERE id = ?1",
        params![id],
        |row| {
            Ok(Device {
                id: row.get(0)?,
                zid: row.get(1)?,
                label: row.get(2)?,
                customer_id: row.get(3)?,
                created_at: row.get(4)?,
                last_seen_at: row.get(5)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GatewayError::NotFound,
        other => GatewayError::Db(other),
    })
}

pub fn delete(conn: &Connection, id: i64) -> Result<bool, GatewayError> {
    let n = conn.execute("DELETE FROM devices WHERE id = ?1", params![id])?;
    Ok(n > 0)
}
