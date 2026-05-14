//! `POST /v1/tunnels` — opens a new public listener for `kind = tcp`
//! or registers a host route for `kind = http`.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use crate::auth::middleware::AuthedUser;
use crate::db::{devices, tunnels};
use crate::error::GatewayError;
use crate::state::AppState;
use crate::tunnel::manager::TunnelEvent;

#[derive(Deserialize)]
pub struct CreateTunnel {
    pub device_id: i64,
    pub kind: String,
    pub local_port: i64,
    pub public_hostname: Option<String>,
    pub public_port: Option<i64>,
}

pub async fn handler(
    State(state): State<AppState>,
    AuthedUser(_caller): AuthedUser,
    Json(body): Json<CreateTunnel>,
) -> Result<(axum::http::StatusCode, Json<tunnels::Tunnel>), GatewayError> {
    let db = state.db.clone();
    let conn = db.get()?;
    let tunnel = tokio::task::spawn_blocking(move || {
        tunnels::insert(
            &conn,
            body.device_id,
            &body.kind,
            body.local_port,
            body.public_hostname.as_deref(),
            body.public_port,
        )
    })
    .await
    .unwrap()?;

    // Hot-start the TCP listener if applicable.
    if tunnel.kind == "tcp" {
        if let Some(public_port) = tunnel.public_port {
            let conn = db.get()?;
            let tid = tunnel.id;
            let did = tunnel.device_id;
            let lp = tunnel.local_port;
            if let Ok(device) = tokio::task::spawn_blocking(move || devices::get(&conn, did)).await.unwrap() {
                let twz = tunnels::TunnelWithZid {
                    id: tid,
                    zid: device.zid,
                    kind: "tcp".into(),
                    local_port: lp as u16,
                    public_port: public_port as u16,
                    enabled: true,
                };
                let _ = state.tunnel_tx.send(TunnelEvent::Added(twz)).await;
            }
        }
    }

    Ok((axum::http::StatusCode::CREATED, Json(tunnel)))
}
