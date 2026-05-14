//! `DELETE /v1/devices/:id` — cascades to `tunnels` via the FK.

use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::auth::middleware::AuthedUser;
use crate::db::devices;
use crate::error::GatewayError;
use crate::state::AppState;

pub async fn handler(
    State(state): State<AppState>,
    AuthedUser(_caller): AuthedUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, GatewayError> {
    let conn = state.db.get()?;
    let deleted = tokio::task::spawn_blocking(move || devices::delete(&conn, id))
        .await
        .unwrap()?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(GatewayError::NotFound)
    }
}
