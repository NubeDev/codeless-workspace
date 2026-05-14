//! `GET /v1/log` — cursor-paginated query over the persisted log
//! ring. Read-only, newest-first.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::middleware::AuthedUser;
use crate::db::logs::{self, LogRow};
use crate::error::GatewayError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub device: Option<i64>,
    pub topic: Option<String>,
    pub level: Option<String>,
    pub since: Option<i64>,
    pub cursor: Option<i64>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    100
}

#[derive(Debug, Serialize)]
pub struct LogsPage {
    pub entries: Vec<LogRow>,
    pub next_cursor: Option<i64>,
}

pub async fn handler(
    State(state): State<AppState>,
    AuthedUser(_caller): AuthedUser,
    Query(q): Query<LogsQuery>,
) -> Result<Json<LogsPage>, GatewayError> {
    let conn = state.db.get()?;
    let limit = q.limit;
    let entries = tokio::task::spawn_blocking(move || {
        logs::list(
            &conn,
            q.device,
            q.topic.as_deref(),
            q.level.as_deref(),
            q.since,
            q.cursor,
            limit,
        )
    })
    .await
    .unwrap()?;

    let next_cursor = if entries.len() as i64 >= limit {
        entries.last().map(|r| r.id)
    } else {
        None
    };
    Ok(Json(LogsPage {
        entries,
        next_cursor,
    }))
}
