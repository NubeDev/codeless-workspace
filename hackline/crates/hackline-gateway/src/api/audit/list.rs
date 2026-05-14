//! `GET /v1/audit?limit=N` — recent audit entries.

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::auth::middleware::AuthedUser;
use crate::db::audit;
use crate::error::GatewayError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct AuditQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

pub async fn handler(
    State(state): State<AppState>,
    AuthedUser(_caller): AuthedUser,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<audit::AuditEntry>>, GatewayError> {
    let conn = state.db.get()?;
    let entries = tokio::task::spawn_blocking(move || audit::list_recent(&conn, q.limit))
        .await
        .unwrap()?;
    Ok(Json(entries))
}
