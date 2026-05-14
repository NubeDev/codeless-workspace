//! `GET /v1/devices/:id`.

use axum::extract::{Path, State};
use axum::Json;

use crate::db::devices;
use crate::error::GatewayError;
use crate::state::AppState;

pub async fn handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<devices::Device>, GatewayError> {
    let conn = state.db.get()?;
    let device = tokio::task::spawn_blocking(move || devices::get(&conn, id))
        .await
        .unwrap()?;
    Ok(Json(device))
}
