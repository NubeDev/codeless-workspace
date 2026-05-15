//! `GET /v1/audit?limit=N` — recent audit entries.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

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

/// Page envelope matches the TS `Page<T>` contract used by every
/// other listing endpoint (cmd outbox, etc.). The audit list is not
/// yet cursor-paginated server-side, so `next_cursor` is always
/// `None`; the wrapper exists so the client never has to special-case
/// this endpoint and `page.entries` is always defined.
#[derive(Serialize)]
pub struct AuditPage {
    pub entries: Vec<audit::AuditEntry>,
    pub next_cursor: Option<String>,
}

pub async fn handler(
    State(state): State<AppState>,
    AuthedUser(caller): AuthedUser,
    Query(q): Query<AuditQuery>,
) -> Result<Json<AuditPage>, GatewayError> {
    let conn = state.db.get()?;
    let org_id = caller.org_id;
    let entries = tokio::task::spawn_blocking(move || audit::list_recent(&conn, org_id, q.limit))
        .await
        .unwrap()?;
    Ok(Json(AuditPage {
        entries,
        next_cursor: None,
    }))
}
