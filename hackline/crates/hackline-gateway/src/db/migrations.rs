//! Embedded refinery migrations from `../../migrations/`. Run on
//! every boot; idempotent.

use rusqlite::Connection;

const V001_INIT: &str = include_str!("../../migrations/V001__init.sql");

/// Run all migrations. Idempotent — uses a `meta` table to track
/// which migrations have been applied.
pub fn run(conn: &Connection) -> Result<(), crate::error::GatewayError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version INTEGER PRIMARY KEY,
            name    TEXT NOT NULL,
            applied_at INTEGER NOT NULL
        );"
    ).map_err(|e| crate::error::GatewayError::Db(e))?;

    let applied: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM _migrations WHERE version = 1",
            [],
            |row| row.get(0),
        )
        .map_err(crate::error::GatewayError::Db)?;

    if !applied {
        conn.execute_batch(V001_INIT)
            .map_err(crate::error::GatewayError::Db)?;
        conn.execute(
            "INSERT INTO _migrations (version, name, applied_at) VALUES (1, 'V001__init', unixepoch())",
            [],
        ).map_err(crate::error::GatewayError::Db)?;
        tracing::info!("applied migration V001__init");
    }
    Ok(())
}
