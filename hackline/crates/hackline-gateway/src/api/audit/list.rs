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

/// Public projection of `db::audit::AuditEntry` matching
/// `DOCS/openapi.yaml` §AuditEntry. The DB row carries
/// `tunnel.session`-shaped extras (`ts_close`, `bytes_up`,
/// `bytes_down`, `peer`, `request_id`) that are an internal
/// implementation detail; the wire only documents the
/// point-in-time fields plus a `subject` derived from whichever FK
/// the row references.
#[derive(Serialize)]
pub struct AuditEntryView {
    pub id: i64,
    pub at: i64,
    pub actor_user_id: Option<i64>,
    pub action: String,
    pub subject: String,
    pub detail: serde_json::Value,
}

/// Page envelope per `DOCS/openapi.yaml` §AuditPage:
/// `{ items, next_cursor }`. Audit list is not yet cursor-paginated
/// so `next_cursor` is always `None`; the wrapper exists so a future
/// pagination addition is non-breaking. Other paginated endpoints
/// (events, cmd outbox, logs) share the same envelope shape.
#[derive(Serialize)]
pub struct AuditPage {
    pub items: Vec<AuditEntryView>,
    pub next_cursor: Option<i64>,
}

pub async fn handler(
    State(state): State<AppState>,
    AuthedUser(caller): AuthedUser,
    Query(q): Query<AuditQuery>,
) -> Result<Json<AuditPage>, GatewayError> {
    let conn = state.db.get()?;
    let org_id = caller.org_id;
    let rows = tokio::task::spawn_blocking(move || audit::list_recent(&conn, org_id, q.limit))
        .await
        .unwrap()?;
    let items = rows.into_iter().map(project).collect();
    Ok(Json(AuditPage {
        items,
        next_cursor: None,
    }))
}

/// `subject = "<resource>:<id>"` per openapi. Tunnel beats device
/// beats user — the most specific FK wins so an audit reader can
/// jump straight to the entity that caused the row. Rows with none
/// of the three (e.g. `auth.login` before user resolution) emit an
/// empty subject; openapi requires the field but not non-empty.
fn project(row: audit::AuditEntry) -> AuditEntryView {
    let subject = if let Some(t) = row.tunnel_id {
        format!("tunnel:{t}")
    } else if let Some(d) = row.device_id {
        format!("device:{d}")
    } else if let Some(u) = row.user_id {
        format!("user:{u}")
    } else {
        String::new()
    };
    let detail = match row.detail {
        Some(s) => serde_json::from_str(&s).unwrap_or_else(|_| {
            // The DB column has historically held free-form text as
            // well as JSON. Wrap non-JSON in a documented envelope so
            // the openapi `type: object` invariant holds.
            serde_json::json!({ "raw": s })
        }),
        None => serde_json::Value::Null,
    };
    AuditEntryView {
        id: row.id,
        at: row.ts,
        actor_user_id: row.user_id,
        action: row.action,
        subject,
        detail,
    }
}

