//! Process-wide application state passed to every axum handler.
//! Holds the db pool, the Zenoh session, the in-process events bus,
//! and the tunnel manager. Concrete (no `dyn`) — tests build a real
//! one against a loopback Zenoh router rather than mocking.

use std::sync::Arc;

use crate::db::pool::DbPool;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub zenoh: Arc<zenoh::Session>,
}
