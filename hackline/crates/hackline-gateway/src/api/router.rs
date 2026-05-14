//! Builds the axum `Router` by composing every handler in this
//! module tree. The only file that knows the full URL surface.

use axum::routing::{delete, get, post};
use axum::Router;

use crate::state::AppState;

pub fn build(state: AppState) -> Router {
    Router::new()
        // Unauthenticated
        .route("/v1/health", get(super::health::get))
        .route("/v1/claim/status", get(super::claim::status::handler))
        .route("/v1/claim", post(super::claim::post::handler))
        // Authenticated
        .route("/v1/devices", get(super::devices::list::handler))
        .route("/v1/devices", post(super::devices::create::handler))
        .route("/v1/devices/{id}", get(super::devices::get::handler))
        .route("/v1/devices/{id}", delete(super::devices::delete::handler))
        .route("/v1/tunnels", get(super::tunnels::list::handler))
        .route("/v1/tunnels", post(super::tunnels::create::handler))
        .route("/v1/tunnels/{id}", delete(super::tunnels::delete::handler))
        .route("/v1/users", get(super::users::list::handler))
        .route("/v1/users", post(super::users::create::handler))
        .route("/v1/users/{id}", delete(super::users::delete::handler))
        .route("/v1/users/{id}/tokens", post(super::users::mint_token::handler))
        .route("/v1/audit", get(super::audit::list::handler))
        // Message plane
        .route("/v1/events", get(super::events::list::handler))
        .route("/v1/events/stream", get(super::events::stream::handler))
        .route("/v1/log", get(super::logs::list::handler))
        .route("/v1/log/stream", get(super::logs::stream::handler))
        // Cmd outbox
        .route("/v1/devices/{id}/cmd/{topic}", post(super::cmd::send::handler))
        .route("/v1/devices/{id}/cmd", get(super::cmd::list::handler))
        .route("/v1/cmd/{cmd_id}", delete(super::cmd::cancel::handler))
        // Synchronous RPC
        .route(
            "/v1/devices/{id}/api/{topic}",
            post(super::api_call::call::handler),
        )
        .with_state(state)
}
