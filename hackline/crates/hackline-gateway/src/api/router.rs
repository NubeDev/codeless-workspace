//! Builds the axum `Router` by composing every handler in this
//! module tree. The only file that knows the full URL surface.

use axum::routing::{delete, get, post};
use axum::Router;

use crate::state::AppState;

pub fn build(state: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(super::health::get))
        .route("/v1/devices", get(super::devices::list::handler))
        .route("/v1/devices", post(super::devices::create::handler))
        .route("/v1/devices/{id}", get(super::devices::get::handler))
        .route("/v1/devices/{id}", delete(super::devices::delete::handler))
        .route("/v1/tunnels", get(super::tunnels::list::handler))
        .route("/v1/tunnels", post(super::tunnels::create::handler))
        .route("/v1/tunnels/{id}", delete(super::tunnels::delete::handler))
        .with_state(state)
}
