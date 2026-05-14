//! `GET /v1/devices` — list devices visible to the caller's scope.

use axum::extract::State;
use axum::Json;

use crate::db::devices;
use crate::error::GatewayError;
use crate::state::AppState;

pub async fn handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<devices::Device>>, GatewayError> {
    let conn = state.db.get()?;
    let list = tokio::task::spawn_blocking(move || devices::list(&conn))
        .await
        .unwrap()?;
    Ok(Json(list))
}
