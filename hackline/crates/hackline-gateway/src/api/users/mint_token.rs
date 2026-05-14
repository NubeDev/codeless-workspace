//! `POST /v1/users/:id/tokens` — issue a new token for an existing
//! user. Returns the raw token once; only the hash is persisted.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use crate::auth::middleware::AuthedUser;
use crate::auth::token;
use crate::db::users;
use crate::error::GatewayError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct MintTokenResponse {
    pub token: String,
}

pub async fn handler(
    State(state): State<AppState>,
    AuthedUser(caller): AuthedUser,
    Path(user_id): Path<i64>,
) -> Result<(StatusCode, Json<MintTokenResponse>), GatewayError> {
    if caller.role != "owner" && caller.id != user_id {
        return Err(GatewayError::Unauthorized("not permitted".into()));
    }
    let conn = state.db.get()?;
    let raw = tokio::task::spawn_blocking(move || {
        let pair = token::generate();
        users::update_token_hash(&conn, user_id, &pair.hash)?;
        Ok::<_, GatewayError>(pair.raw)
    })
    .await
    .unwrap()?;
    Ok((
        StatusCode::CREATED,
        Json(MintTokenResponse { token: raw }),
    ))
}
