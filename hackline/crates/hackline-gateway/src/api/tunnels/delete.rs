//! `DELETE /v1/tunnels/:id` — closes the listener and deletes the row.

use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::db::tunnels;
use crate::error::GatewayError;
use crate::state::AppState;

pub async fn handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, GatewayError> {
    let conn = state.db.get()?;
    let deleted = tokio::task::spawn_blocking(move || tunnels::delete(&conn, id))
        .await
        .unwrap()?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(GatewayError::NotFound)
    }
}
