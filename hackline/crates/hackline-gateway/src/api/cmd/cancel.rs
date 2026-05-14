//! `DELETE /v1/cmd/:cmd_id` — cancel a queued (not yet delivered)
//! command. Returns 404 if the row was already delivered or never
//! existed; cancel is best-effort, mirroring SCOPE.md §5.3.

use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::auth::middleware::AuthedUser;
use crate::auth::scope;
use crate::db::cmd_outbox;
use crate::error::GatewayError;
use crate::state::AppState;

pub async fn handler(
    State(state): State<AppState>,
    AuthedUser(user): AuthedUser,
    Path(cmd_id): Path<String>,
) -> Result<StatusCode, GatewayError> {
    let db = state.db.clone();
    let cmd_id_lookup = cmd_id.clone();
    let row = tokio::task::spawn_blocking(move || {
        let conn = db.get()?;
        cmd_outbox::get_by_cmd_id(&conn, &cmd_id_lookup)
    })
    .await
    .map_err(|e| GatewayError::Config(format!("blocking task join: {e}")))??;
    let row = row.ok_or(GatewayError::NotFound)?;
    scope::check_device(&user, row.device_id)?;

    let db = state.db.clone();
    let deleted = tokio::task::spawn_blocking(move || {
        let conn = db.get()?;
        cmd_outbox::cancel(&conn, &cmd_id)
    })
    .await
    .map_err(|e| GatewayError::Config(format!("blocking task join: {e}")))??;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(GatewayError::NotFound)
    }
}
