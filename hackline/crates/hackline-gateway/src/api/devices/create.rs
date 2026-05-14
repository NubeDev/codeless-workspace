//! `POST /v1/devices` — register a device by ZID.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;

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
    Json(body): Json<CreateDevice>,
) -> Result<(axum::http::StatusCode, Json<devices::Device>), GatewayError> {
    let conn = state.db.get()?;
    let device = tokio::task::spawn_blocking(move || devices::insert(&conn, &body.zid, &body.label))
        .await
        .unwrap()?;
    Ok((axum::http::StatusCode::CREATED, Json(device)))
}
