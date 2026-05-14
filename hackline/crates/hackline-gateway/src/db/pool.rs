//! r2d2 pool setup. Opens SQLite with WAL mode and the foreign-key
//! pragma. Sized conservatively against tokio's blocking-thread pool.

use std::path::Path;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

pub type DbPool = Pool<SqliteConnectionManager>;

/// Open (or create) a SQLite database and return a connection pool.
pub fn open(path: &Path) -> Result<DbPool, crate::error::GatewayError> {
    let manager = SqliteConnectionManager::file(path)
        .with_init(|conn| {
            conn.execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA foreign_keys = ON;
                 PRAGMA busy_timeout = 5000;",
            )?;
            Ok(())
        });
    let pool = Pool::builder()
        .max_size(16)
        .build(manager)
        .map_err(|e| crate::error::GatewayError::Config(format!("db pool: {e}")))?;
    Ok(pool)
}
