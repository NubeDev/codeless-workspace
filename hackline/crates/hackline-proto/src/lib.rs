//! Wire types and key-expression builders shared by every hackline
//! component. Pure types only — no I/O, no async, no filesystem.

pub mod agent_info;
pub mod connect;
pub mod error;
pub mod event;
pub mod keyexpr;
pub mod zid;
