//! `POST /v1/claim` — atomic consume-pending + insert-owner.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::db::claim;
use crate::error::GatewayError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ClaimRequest {
    pub token: String,
    #[serde(default = "default_owner_name")]
    pub name: String,
}

fn default_owner_name() -> String {
    "owner".into()
}

#[derive(Serialize)]
pub struct ClaimResponse {
    pub user_id: i64,
    pub token: String,
}

pub async fn handler(
    State(state): State<AppState>,
    Json(body): Json<ClaimRequest>,
) -> Result<(StatusCode, Json<ClaimResponse>), GatewayError> {
    let conn = state.db.get()?;
    let (bearer_raw, user_id) = tokio::task::spawn_blocking(move || {
        let bearer = claim::consume(&conn, &body.token, &body.name)?;
        let user = crate::db::users::list(&conn)?
            .into_iter()
            .find(|u| u.role == "owner")
            .ok_or_else(|| GatewayError::BadRequest("claim failed".into()))?;
        Ok::<_, GatewayError>((bearer.raw, user.id))
    })
    .await
    .unwrap()?;

    Ok((
        StatusCode::CREATED,
        Json(ClaimResponse {
            user_id,
            token: bearer_raw,
        }),
    ))
}
