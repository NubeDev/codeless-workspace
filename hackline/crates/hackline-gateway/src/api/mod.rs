//! REST + SSE surface. One file per (resource, verb); the full
//! catalogue is in `DOCS/REST-API.md`. The router wiring lives in
//! `router.rs`.

pub mod audit;
pub mod claim;
pub mod devices;
pub mod events;
pub mod health;
pub mod logs;
pub mod router;
pub mod tunnels;
pub mod users;
