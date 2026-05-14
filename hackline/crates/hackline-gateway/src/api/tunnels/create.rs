//! `POST /v1/tunnels` — opens a new public listener for `kind = tcp`
//! or registers a host route for `kind = http`.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use crate::db::tunnels;
use crate::error::GatewayError;
use crate::state::AppState;

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
    Json(body): Json<CreateTunnel>,
) -> Result<(axum::http::StatusCode, Json<tunnels::Tunnel>), GatewayError> {
    let conn = state.db.get()?;
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
    Ok((axum::http::StatusCode::CREATED, Json(tunnel)))
}
