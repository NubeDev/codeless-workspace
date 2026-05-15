//! `GET /v1/devices/:id/health` — liveliness summary derived from
//! the device's `last_seen_at` column.
//!
//! Shape matches `DOCS/openapi.yaml` `DeviceHealth`:
//! `{ online: bool, last_seen_at: int|null, rtt_ms: int|null }`.
//! The `online` flag flips false when no liveliness probe has
//! arrived within `ONLINE_STALE_SECS` of `now()`. `rtt_ms` is the
//! latency of the last Zenoh `liveliness::Get` round-trip — null
//! when offline or when no probe has been issued yet. Today the
//! gateway does not run a synchronous probe inside this handler
//! (see `liveliness.rs` for the background subscriber that bumps
//! `last_seen_at`); the field is reserved for when that lands so
//! the wire shape does not have to change again.

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

use crate::auth::middleware::AuthedUser;
use crate::db::devices;
use crate::error::GatewayError;
use crate::state::AppState;

/// A device is considered online if the liveliness subscriber has
/// stamped `last_seen_at` within this window. Matches the bridge
/// keepalive period documented in `SCOPE.md` §6 (default 30 s) plus
/// one missed beat of slack.
const ONLINE_STALE_SECS: i64 = 60;

#[derive(Debug, Serialize)]
pub struct DeviceHealth {
    pub online: bool,
    pub last_seen_at: Option<i64>,
    pub rtt_ms: Option<i64>,
}

pub async fn handler(
    State(state): State<AppState>,
    AuthedUser(caller): AuthedUser,
    Path(id): Path<i64>,
) -> Result<Json<DeviceHealth>, GatewayError> {
    let conn = state.db.get()?;
    let org_id = caller.org_id;
    let device = tokio::task::spawn_blocking(move || devices::get_in_org(&conn, org_id, id))
        .await
        .unwrap()?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let online = match device.last_seen_at {
        Some(seen) => now.saturating_sub(seen) <= ONLINE_STALE_SECS,
        None => false,
    };

    Ok(Json(DeviceHealth {
        online,
        last_seen_at: device.last_seen_at,
        rtt_ms: None,
    }))
}
