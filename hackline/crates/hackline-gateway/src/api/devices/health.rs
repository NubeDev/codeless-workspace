//! `GET /v1/devices/:id/health` — liveliness summary derived from
//! the device's `last_seen_at` column plus a synchronous Zenoh
//! `liveliness::Get` probe for `rtt_ms`.
//!
//! Shape matches `DOCS/openapi.yaml` `DeviceHealth`:
//! `{ online: bool, last_seen_at: int|null, rtt_ms: int|null }`.
//! `online` flips false when no liveliness sample has arrived
//! within `ONLINE_STALE_SECS` of `now()` (the background fan-in in
//! `liveliness.rs` is the writer). `rtt_ms` is the measured
//! round-trip of a single liveliness query against the device's
//! own `hackline/<org>/<zid>/health` token; null if the probe
//! times out, errors, or returns no replies.

use std::time::{Duration, Instant};

use axum::extract::{Path, State};
use axum::Json;
use hackline_proto::{keyexpr, Zid};
use serde::Serialize;

use crate::auth::middleware::AuthedUser;
use crate::db::{devices, orgs};
use crate::error::GatewayError;
use crate::state::AppState;

/// A device is considered online if the liveliness subscriber has
/// stamped `last_seen_at` within this window. Matches the bridge
/// keepalive period documented in `SCOPE.md` §6 (default 30 s) plus
/// one missed beat of slack.
const ONLINE_STALE_SECS: i64 = 60;

/// Hard cap on the synchronous probe. Liveliness queries should
/// resolve in single-digit ms on a healthy mesh; 250 ms is enough
/// slack for a slow path while keeping the API endpoint snappy
/// (callers polling this in the UI cannot tolerate seconds).
const PROBE_TIMEOUT_MS: u64 = 250;

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
    let (device, org) = tokio::task::spawn_blocking(move || -> Result<_, GatewayError> {
        let d = devices::get_in_org(&conn, org_id, id)?;
        let o = orgs::get(&conn, org_id)?;
        Ok((d, o))
    })
    .await
    .map_err(|e| GatewayError::Config(format!("blocking task join: {e}")))??;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let online = match device.last_seen_at {
        Some(seen) => now.saturating_sub(seen) <= ONLINE_STALE_SECS,
        None => false,
    };

    let rtt_ms = probe_rtt_ms(&state, &org.slug, &device.zid).await;

    Ok(Json(DeviceHealth {
        online,
        last_seen_at: device.last_seen_at,
        rtt_ms,
    }))
}

/// Issue one `liveliness::Get` against the device's own health
/// token and return the wall-clock RTT to the first reply.
///
/// Errors and timeouts collapse to `None` rather than failing the
/// HTTP request: an unreachable device should still answer
/// `GET /v1/devices/:id/health` with `online: false, rtt_ms: null`,
/// not 500. A malformed `zid` (cannot construct the keyexpr) also
/// returns `None`; the device row is the source of truth and a
/// shape error there is an internal data issue, not a probe
/// failure to surface to the caller.
async fn probe_rtt_ms(
    state: &AppState,
    org_slug: &str,
    zid_str: &str,
) -> Option<i64> {
    let zid = Zid::new(zid_str).ok()?;
    let ke = keyexpr::health(org_slug, &zid);

    let started = Instant::now();
    let replies = state
        .zenoh
        .liveliness()
        .get(&ke)
        .timeout(Duration::from_millis(PROBE_TIMEOUT_MS))
        .await
        .ok()?;

    // `recv_async` returns the next reply or an error when the
    // handler is closed by the timeout above. Either way the
    // probe is over once we've waited that long, so wrap the
    // recv in the same hard cap as a belt-and-braces guard
    // against a future zenoh change that holds the channel open
    // past its declared timeout.
    let recv = tokio::time::timeout(
        Duration::from_millis(PROBE_TIMEOUT_MS),
        replies.recv_async(),
    )
    .await
    .ok()?
    .ok()?;
    drop(recv);

    Some(started.elapsed().as_millis() as i64)
}

