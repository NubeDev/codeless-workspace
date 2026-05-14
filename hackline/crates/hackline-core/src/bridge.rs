//! Bidirectional byte copy between a `TcpStream` and a Zenoh
//! sample stream. The exact Zenoh API shape (streaming-reply query
//! vs. paired pub/sub) is decided in Phase 1; this module owns both
//! variants behind a single trait so callers don't care which won.
