//! `GET /v1/devices/:id/cmd?status=...&cursor=...&limit=...` —
//! cursor-paginated outbox listing.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::middleware::AuthedUser;
use crate::auth::scope;
use crate::db::cmd_outbox::{self, CmdRow};
use crate::error::GatewayError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
    pub cursor: Option<i64>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    100
}

#[derive(Serialize)]
pub struct CmdPage {
    pub entries: Vec<CmdRow>,
    pub next_cursor: Option<i64>,
}

pub async fn handler(
    State(state): State<AppState>,
    AuthedUser(user): AuthedUser,
    Path(device_id): Path<i64>,
    Query(q): Query<ListQuery>,
) -> Result<Json<CmdPage>, GatewayError> {
    scope::check_device(&user, device_id)?;
    let conn = state.db.get()?;
    let status = q.status.clone();
    let limit = q.limit;
    let cursor = q.cursor;
    let entries = tokio::task::spawn_blocking(move || {
        cmd_outbox::list_by_device(&conn, device_id, status.as_deref(), cursor, limit)
    })
    .await
    .map_err(|e| GatewayError::Config(format!("blocking task join: {e}")))??;

    let next_cursor = if entries.len() as i64 >= limit {
        entries.last().map(|r| r.id)
    } else {
        None
    };
    Ok(Json(CmdPage {
        entries,
        next_cursor,
    }))
}
