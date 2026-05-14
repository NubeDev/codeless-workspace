//! In-process broadcast bus that fans gateway events out to every
//! connected SSE subscriber. Backed by `tokio::sync::broadcast`;
//! lagging subscribers see a `Lagged` error and reconnect.
