//! `GET /v1/tunnels`.

use axum::extract::State;
use axum::Json;

use crate::db::tunnels;
use crate::error::GatewayError;
use crate::state::AppState;

pub async fn handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<tunnels::Tunnel>>, GatewayError> {
    let conn = state.db.get()?;
    let list = tokio::task::spawn_blocking(move || tunnels::list(&conn))
        .await
        .unwrap()?;
    Ok(Json(list))
}
