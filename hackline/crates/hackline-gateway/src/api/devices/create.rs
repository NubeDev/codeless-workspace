//! `POST /v1/devices` — register a device by ZID.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use crate::auth::middleware::AuthedUser;
use crate::db::devices;
use crate::error::GatewayError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateDevice {
    pub zid: String,
    pub label: String,
}

pub async fn handler(
    State(state): State<AppState>,
    AuthedUser(caller): AuthedUser,
    Json(body): Json<CreateDevice>,
) -> Result<(axum::http::StatusCode, Json<devices::Device>), GatewayError> {
    let conn = state.db.get()?;
    let org_id = caller.org_id;
    let device = tokio::task::spawn_blocking(move || devices::insert(&conn, org_id, &body.zid, &body.label))
        .await
        .unwrap()?;
    Ok((axum::http::StatusCode::CREATED, Json(device)))
}
